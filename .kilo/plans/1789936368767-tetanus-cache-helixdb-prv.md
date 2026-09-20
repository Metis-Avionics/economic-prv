# Plan: PRV — TETANUS Enforcement, thesix Cache Integration, HelixDB Testing & Parallelism Spec

## Goal

Bridge the four infrastructure gaps identified in the PRV/NS-Pro comparison analysis:

1. **TETANUS Enforcement** — materialize the "Power of 10" enforcement system (config + Python checker + workspace lints) that currently exists only as an aspirational reference in PRV's plan file.
2. **Apalis + Rayon** — define the parallelism spec (`specs/parallelism.toml`) with the same two-tier (strict/advisory) rules as NS-Pro; defer actual implementation (no numerical hot path needs parallelism yet).
3. **CLI / Ingestion** — refactor `prv-cli living update` and `session handover` to project from `living.toml` instead of emitting hardcoded strings; remove the unused `thesix` from `crates/cli`; wire `thesix` into a real cache crate.
4. **HelixDB Testing & Cache** — create `SCHEMA.md`, `scripts/check_schema.sh`, CI gate; add `crates/cache` with `thesix` integration.

## Current State Snapshot

| Area | PRV | NS-Pro (reference) |
|---|---|---|
| `tetanus.toml` | absent | 164-line config |
| `clippy.toml` | absent | thresholds (60 lines, complexity 25) |
| `scripts/tetanus-check.py` | absent | 507-line static checker |
| `deny.toml` | absent | 82-line supply-chain policy |
| `.cargo/audit.toml` | absent | advisory exceptions |
| `SCHEMA.md` | absent | 24-line HelixDB schema sketch |
| `scripts/check_schema.sh` | absent | 15-line schema gate |
| CI workflows | none | full ci.yml with 5 jobs |
| `#[lints] workspace = true` | **none** in any crate | every crate |
| `[workspace.lints]` | absent | two-tier deny/warn |
| `thesix` usage | declared in Cargo.toml + cli/Cargo.toml, **zero `.rs` usage** | used in `crates/ns-cache/src/manager.rs` |
| `helix-db` | not a dependency | referenced in AGENTS.md |
| `crates/cli/src/main.rs` | 429 lines, `living update`/`session handover` are hardcoded strings | — |

## Key Decisions

1. **Strict crates** (deny profile, checked by Python checker): `prv-core`, `prv-filter`, `prv-monte-carlo`, `prv-policy`, `prv-geometry`. These own deterministic numerics on the EKF/step path.
2. **Advisory crates** (warn profile, NOT checked by Python checker): `prv-data`, `prv-evaluation`, `prv-cli`, `crates/cache` (new).
3. **hot_fns for TETANUS rule 3** (no heap allocation): `predict`, `update`, `jacobian_f`, `jacobian_h`, `stabilize_covariance`, `quaternion_normalize`. Pure numerical step functions on the EKF path. Batch drivers (`simulate`, `evaluate`) are NOT in hot_fns — they preallocate results by design.
4. **New `crates/cache`** crate for thesix integration, following NS-Pro's `ns-cache/manager.rs` pattern (six in-memory stub tiers + Cachelito policy + DashMap stats/index).
5. **Apalis + Rayon are deferred** — no Monte Carlo batch parallelism requirement has emerged; `specs/parallelism.toml` codifies the rules for when it does.
6. **CI uses `ubuntu-latest`** (hosted) — PRV has no dioxus-desktop/webkit system-lib chain.
7. **helix-db Rust SDK NOT added yet** — "retained but not central" per original plan. HelixDB integration starts with SCHEMA.md + check_schema.sh (text-only gates). Adding the Rust SDK is a follow-up when PRV needs programmatic queries.

## Identified Violations (strict crates)

The following violations exist in PRV's strict crates and must be remediated before CI can pass:

### power10-07 (no unwrap/expect/panic in strict lib code)

| File:Line | Violation | Remediation |
|---|---|---|
| `crates/monte_carlo/src/simulator.rs:64` | `.expect("Cholesky decomposition after eigenvalue clipping")` in `sample_state` | Change to `unwrap_or_else` returning empty `Vec::new()`, or change `sample_state` to return `Result` |
| `crates/monte_carlo/src/simulator.rs:129` | `.expect(...)` in `simulate` | Same as above |
| `crates/monte_carlo/src/simulator.rs:162` | `.unwrap()` on `p.last().cloned()` | Use `unwrap_or(&State::default().clone())` or filter |
| `crates/policy/src/engine.rs:232` | `Normal::new(0.0, 0.1).unwrap()` in `sample_noise` | Replace with `0.1 * rng.sample(StandardNormal)` (eliminates `Normal::new` entirely) |

### power10-04 (function > 60 lines)

| File:Function | Lines | Remediation |
|---|---|---|
| `crates/monte_carlo/src/simulator.rs:simulate` | ~135 | Split into `prepare_cholesky`, `run_paths`, `compute_results` |
| `crates/policy/src/engine.rs:evaluate` | ~140 | Split into `compute_scores`, `build_distribution`, `check_constraints` |

### power10-03 (heap allocation in hot fns) — to be verified after hot_fns set

`sample_state` and `run_path` use `Vec::with_capacity` / `vec![`. If included in hot_fns, these need `TETANUS-exempt` or buffer refactoring. **Decision: exclude from hot_fns** (they are batch samplers, not step functions).

## Ordered Task List

> **Stage ordering matters.** C11 (tokio/dashmap workspace deps) must precede C2 (cache crate). C must precede D5 (test lives in `crates/cache/tests/`). A must precede E2 (CLI validation references tetanus.toml). G (CI) must come last. H (AGENTS.md) depends on A and D. I (living.toml) is final.

### Stage A: TETANUS Enforcement Infrastructure

- [ ] **A1**: Create `scripts/tetanus-check.py` — copy from `<NS-Pro>/scripts/tetanus-check.py`, modify `check_plumbing()` to remove the hardcoded `"scope"` member (replace with dynamic workspace member discovery from `Cargo.toml`). Verify `ROOT` resolves to PRV project root.
- [ ] **A2**: Create `clippy.toml` at project root — adapt from NS-Pro:
  ```toml
  too-many-lines-threshold = 60
  cognitive-complexity-threshold = 25
  allow-unwrap-in-tests = true
  allow-expect-in-tests = true
  allow-panic-in-tests = true
  ```
- [ ] **A3**: Create `tetanus.toml` at project root — adapt from NS-Pro with PRV-specific scope:
  - `[scope]` strict = `["crates/core", "crates/filter", "crates/monte_carlo", "crates/policy", "crates/geometry"]`, advisory = `["crates/data", "crates/evaluation", "crates/cli", "crates/cache"]`
  - `[thresholds]` max_fn_lines=60, min_asserts_per_fn_avg=2, max_non_test_cfgs=2, cognitive_complexity=25
  - `[rule3]` hot_fns = `["predict", "update", "jacobian_f", "jacobian_h", "stabilize_covariance", "quaternion_normalize"]`, cold_markers = `["Err", "Error", "reason", "assert"]`
  - `[[rule]]` entries for all 10 Power-of-10 rules, adapted for Rust
  - `[ci]` check/lint/test commands
- [ ] **A4**: Add `[workspace.lints]` section to root `Cargo.toml` — adapt from NS-Pro's `[workspace.lints.rust]` and `[workspace.lints.clippy]`. Key levels: `unsafe_code = "deny"`, `all = "deny"`, pedantic/nursery warn tracks, `panic = "deny"`, `todo = "deny"`, `wildcard_imports = "deny"`, etc.
- [ ] **A5**: Add `[lints]` `workspace = true` to every crate's `Cargo.toml` (all 8 existing + 1 new cache crate). This is a plumbing gate — every workspace member must inherit.
- [ ] **A6**: Update `lib.rs` headers in **strict** crates (core, filter, monte_carlo, policy, geometry) — remove `#![warn(clippy::pedantic, clippy::nursery)]` (inherited from workspace), keep `#![forbid(unsafe_code)]` (escalates workspace `deny` → `forbid`), and add:
  ```rust
  #![deny(clippy::unwrap_used, clippy::expect_used)]
  #![deny(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro, clippy::use_debug)]
  ```
- [ ] **A7**: Update `lib.rs` headers in **advisory** crates (data, evaluation, cache) and `main.rs` in cli — remove `#![warn(clippy::pedantic, clippy::nursery)]` (inherited from workspace); keep `#![forbid(unsafe_code)]`. Do NOT add strict deny headers.
- [ ] **A8**: Remediate all strict-crate violations identified above (4 power10-07, 2 power10-04). See the "Remediation Details" section below.
- [ ] **A9**: Run `python3 scripts/tetanus-check.py` — must report 0 errors.
- [ ] **A10**: Run `cargo clippy --workspace --all-targets` — must pass with 0 errors (workspace denies).
- [ ] **A11**: Update `specs/cli.toml` to add `tetanus_check = true` to `[spec_validate_checks]` (so `prv-cli spec validate` knows about it).

### Stage B: Supply-Chain Gates

- [ ] **B1**: Create `deny.toml` — adapt from NS-Pro:
  - `[advisories]` with `RUSTSEC-2024-0384` and `RUSTSEC-2025-0057` ignored (reached via thesix -> sled -> parking_lot) with full justification
  - `[licenses]` allow-list (MIT, Apache-2.0, BSD, ISC, etc.)
  - `[bans]` deny `openssl-sys`; warn on multiple-versions
  - `[sources]` allow only crates.io registry
- [ ] **B2**: Create `.cargo/audit.toml` — mirror deny.toml advisory exceptions for `cargo-audit`.
- [ ] **B3**: Add `cargo-deny` and `cargo-machete` to CI (check for unused dependencies).

### Stage C: thesix Cache Integration (crates/cache)

- [ ] **C1**: Create `crates/cache/Cargo.toml`:
   - `name = "prv-cache"`, `edition.workspace = true`, `license.workspace = true`
   - Dependencies: `thesix.workspace = true`, `tokio.workspace = true`, `dashmap`, `serde`, `serde_json`, `thiserror`, `tracing`
   - `[lints] workspace = true`
- [ ] **C2**: Create `crates/cache/src/lib.rs`:
  - Module declarations: `pub mod keys; pub mod manager; pub mod stats; pub mod index;`
  - Re-export `NsCache`, key functions, `CacheStats`
  - Advisory crate header (no strict deny)
- [ ] **C3**: Create `crates/cache/src/manager.rs` — adapt from NS-Pro `ns-cache/manager.rs`:
  - `NsCache` struct with `Arc<CacheManager<String, String, DefaultPolicy>>` + stats + index
  - `new()` builds six in-memory stub tiers (L0Stub..L5Stub)
  - `ctx()` / `anonymous_ctx()` for CacheContext
  - `get` / `get_or_fetch` / `set` / `invalidate` / `refresh` / `exists`
  - `warm_many` / `invalidate_many` via `tokio::task::JoinSet`
  - `stats_snapshot()` / `keys_for_subject()`
- [ ] **C4**: Create `crates/cache/src/keys.rs` — PRV-specific cache key functions:
  - `simulation_results_key(case_id: &str) -> String` → `simulation:results:{case_id}`
  - `ekf_state_key(timestamp: &str) -> String` → `ekf:state:{timestamp}`
  - `historical_data_key(source: &str) -> String` → `data:historical:{source}`
  - `policy_decision_key(regime: &str) -> String` → `policy:decision:{regime}`
  - `namespace_of(key: &str) -> &str` for stats grouping
- [ ] **C5**: Create `crates/cache/src/stats.rs` — `CacheStats` with per-namespace operation counters (DashMap-backed, atomic).
- [ ] **C6**: Create `crates/cache/src/index.rs` — `KeyIndex` with DashMap-backed key → subject index.
- [ ] **C7**: Add `crates/cache` to workspace members in root `Cargo.toml` and `spec.toml [workspace].members`.
- [ ] **C8**: Add `prv-cache` as dependency of `prv-data` in `crates/data/Cargo.toml`.
- [ ] **C9**: Integrate cache into `crates/data/src/loader.rs`:
  - `DataLoader::load_historical_cached()` — wrap `load_historical()` with `get_or_fetch` from `NsCache`
  - Cache key: `data:historical:{path_hash}`
  - Write-through on cache hit; populate cache on miss
- [ ] **C10**: Remove `thesix` from `crates/cli/Cargo.toml` (route through `prv-cache` if CLI needs cache access).
- [ ] **C11**: Add `tokio = { version = "1", features = ["full"] }` to workspace dependencies in root `Cargo.toml` (required by `crates/cache` for `JoinSet` and async cache methods; also needed for future Apalis integration).

### Stage D: HelixDB Schema + CI Gate

- [ ] **D1**: Create `SCHEMA.md` at project root — PRV-specific HelixDB schema sketch:
  - Labels: `SimulationCase`, `StateVector`, `ShockSpec`, `MonteCarloPath`, `PolicyDecision`, `EvaluationResult`
  - Edges: `SimulationCase -HAS_STATE-> StateVector`, `SimulationCase -SHOCKED_WITH-> ShockSpec`, `SimulationCase -SIMULATED-> MonteCarloPath`, `SimulationCase -POLICY_APPLIED-> PolicyDecision`, `SimulationCase -EVALUATED_AS-> EvaluationResult`
  - Query patterns: `listCases`, `simulationByRegime`, `policyImpact`
- [ ] **D2**: Create `scripts/check_schema.sh` — bash script that validates SCHEMA.md declares all PRV labels and edges (adapt from NS-Pro's `check_schema.sh`).
- [ ] **D3**: Add `examples/helix-query.json` — a PRV-specific HelixDB query (e.g., counting `SimulationCase` nodes or reading MonteCarloPath).
- [ ] **D4**: Update `helix.toml` — change `container_runtime` to `podman` (if Podman is available) or keep `docker`; keep `[local.prv]` config. Note v0.0.5 quirk from NS-Pro AGENTS.md: store integer properties as f64.
- [ ] **D5**: Add a Rust integration test in `crates/cache/tests/test_schema.rs` that reads `SCHEMA.md` as text and asserts all 6 labels + 5 edges are present (no running HelixDB instance required).

### Stage E: CLI Living Update / Session Handover Refactor

- [ ] **E1**: Create `crates/cli/src/living.rs` — shared module for living.toml parsing and doc generation:
  - `parse_living()` — read + parse `living.toml` via `toml_edit`
  - `generate_living_doc(living, specs) -> String` — project from living.toml [handover] + spec.toml
  - `generate_changelog_doc(living) -> String` — project from living.toml [changelog]
  - `generate_session_doc(living) -> String` — project from living.toml [handover].phase/status
  - `generate_handover_doc(living) -> String` — project from living.toml [handover]
- [ ] **E2**: Refactor `crates/cli/src/main.rs:validate_spec()` — keep existing logic, add check that `tetanus.toml` exists (reference A11).
- [ ] **E3**: Refactor `update_living()` — replace hardcoded strings with calls to `living.rs` generators. Read `living.toml` as the source of truth. Read `spec.toml` for architecture/project overview.
- [ ] **E4**: Refactor `session_handover()` — read current `living.toml`, update `[handover]` and `[changelog]` sections with new handover data, write back to `living.toml`, then regenerate all 4 docs via `living.rs`.
- [ ] **E5**: `spec start` — replace `println!("Starting spec...")` with code that reads `spec.toml` + `specs/*.toml` + `living.toml` and prints structured project state.
- [ ] **E6**: `spec plan` — replace `println!("Planning mode...")` with code that generates a new plan entry and updates `living.toml`.
- [ ] **E7**: `status` — replace static print with `cargo tree`-based crate tree + spec validation status + living.toml status.

### Stage F: Apalis + Rayon Parallelism Spec

- [ ] **F1**: Create `specs/parallelism.toml` — adapt from NS-Pro with PRV-specific rules:
  ```toml
  [spec]
  name = "parallelism"
  version = "0.1.0"
  parent = "spec.toml"
  status = "deferred"
  
  [rules]
  no_threads_in_strict_hot_fns = true
  strict_hot_fns = ["predict", "update", "jacobian_f", "jacobian_h", "stabilize_covariance", "quaternion_normalize"]
  rayon_advisory_only = true
  no_rayon_in_strict_crates = true
  tokio_io_concurrency_only = true
  apalis_memory_backend_only = true
  additive_par_variants = true
  
  [deferred]
  reason = "No batch parallelism requirement yet; Monte Carlo path sampling is embarrassingly parallel but not a bottleneck"
  trigger = "Monte Carlo n_paths > 10_000 or wall-clock > 5s per batch"
  
  [rayon]
  # Pattern to follow when implemented (from NS-Pro):
  # prv_monte_carlo::simulate_par = "par_iter over n_paths, reduce sequentially"
  
  [tokio]
  helix_note = "HelixBackend post() stays sequential per query; batch across cases via JoinSet at caller"
  cache_warm_many = "prv-cache warm_many/invalidate_many via JoinSet"
  
  [apalis]
  # Deferred: only when prv-cli or prv-data has background job requirements
  # (cache sweeps, periodic persistence, scheduled simulations)
  ```

### Stage G: CI Workflow

- [ ] **G1**: Create `.github/workflows/ci.yml` — adapted from NS-Pro, simplified for hosted runner:
  - `changes` job — paths-filter (Rust files, Cargo.toml, Cargo.lock, clippy.toml, tetanus.toml, deny.toml, .cargo/audit.toml, scripts/, SCHEMA.md, specs/)
  - `tetanus-check` job — `python3 scripts/tetanus-check.py` (ubuntu-latest, runs first, fail-fast)
  - `rust-check` job — `cargo fmt --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`, `bash scripts/check_schema.sh`, `cargo deny check`, `cargo machete --skip-target-dir`
  - No self-hosted runner, no NEO eval, no SBOM export (simpler than NS-Pro)
- [ ] **G2**: Add `deny.toml` and `.cargo/audit.toml` to the paths-filter `rust` pattern.

### Stage H: AGENTS.md Update

- [ ] **H1**: Update `AGENTS.md` — add a "PRV Quality Gates" section:
  ```bash
  cargo fmt --check
  python3 scripts/tetanus-check.py          # 0 errors required
  cargo clippy --workspace --all-targets    # workspace all=deny; denies are errors
  cargo test --workspace
  bash scripts/check_schema.sh              # SCHEMA.md labels + edges
  ```
  - Document strict vs advisory crate classification
  - Document TETANUS escape hatch (`// TETANUS-exempt(power10-0X): <reason>`)
  - Document hot_fns list
  - Note prv-cli commands for Spec-and-Go loop

### Stage I: Update living.toml

- [ ] **I1**: Update `living.toml` — add changelog entries for all items created/modified in this plan:
  - `tetanus_infra_created` — tetanus.toml, clippy.toml, scripts/tetanus-check.py
  - `workspace_lints_enabled` — [workspace.lints] + [lints] workspace = true
  - `supply_chain_gates_added` — deny.toml, .cargo/audit.toml
  - `cache_crate_created` — crates/cache with thesix integration
  - `helixdb_schema_defined` — SCHEMA.md + check_schema.sh
  - `cli_living_update_refactored` — projection from living.toml
  - `parallelism_spec_defined` — specs/parallelism.toml (deferred)
  - `ci_workflow_created` — .github/workflows/ci.yml
- [ ] **I2**: Add `remaining` entries for Apalis/Rayon implementation (when triggered).

## Remediation Details (Stage A8)

### monte_carlo/src/simulator.rs

**sample_state (line 55-65):**
```rust
// Before:
let chol = Cholesky::new(*covariance).unwrap_or_else(|| {
    // ... eigenvalue clipping ...
    Cholesky::new(cov).expect("Cholesky decomposition after eigenvalue clipping")
});

// After:
let chol = Cholesky::new(*covariance).unwrap_or_else(|| {
    // ... eigenvalue clipping ...
    // TETANUS-exempt(power10-07): eigenvalue clipping guarantees PSD after fix-up
    Cholesky::new(cov).unwrap_or_default()
});
```
Or better: change `sample_state` to return `Result<Vec<State>, PrvError>` and propagate the Cholesky error. This requires updating callers in `simulate` and tests.

**sample_state (line 162):**
```rust
// Before:
.map(|p| p.last().cloned().unwrap())
// After:
.filter_map(|p| p.last().cloned())
```

**simulate (lines 120-130):** Same pattern as sample_state — extract Cholesky into a helper or use Result.

### policy/src/engine.rs

**sample_noise (line 232):**
```rust
// Before:
let normal = Normal::new(0.0, 0.1).unwrap();
normal.sample(r)

// After:
r.sample(rand_distr::StandardNormal) * 0.1
```
This eliminates the `Normal::new` call entirely. `StandardNormal` is infallible.

### simulate / evaluate function splitting

**simulate:** Extract `prepare_cholesky(covariance) -> Cholesky` helper, `run_single_path(&self, initial, horizon, shocks, l) -> Vec<State>`, and `compute_results(paths, seed, sample_count) -> MonteCarloResults`.

**evaluate:** Extract `compute_bias_scores(bias) -> (f64, f64, f64, f64, f64, f64)` for the 6 instrument scores, `build_distribution(scores, bias, state, weights, rng) -> PolicyDistribution`.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Workspace `all = "deny"` is stricter than current `pedantic + nursery` warns — may surface new violations | Run `cargo clippy --workspace --all-targets -- -D warnings` early; existing `#![allow(...)]` in CLI can be migrated to `TETANUS-exempt` |
| Strict crate lib code has 4 unwrap/expect violations — remediating changes APIs | Plan documents each fix; `sample_state`/`simulate` may need `Result` return type (breaking change for `prv-evaluation` callers) |
| `simulate` and `evaluate` are >60 lines — splitting changes internal structure | Extract helpers as private fns; keep public API unchanged |
| `crates/cache` new crate needs thesix API knowledge | Copy `ns-cache/manager.rs` pattern exactly; thesix 0.2.3 API is confirmed resolved |
| CI `cargo deny check` may flag existing dependencies (sled, parking_lot) | deny.toml includes the same advisory exceptions as NS-Pro |
| No Docker/Podman in sandbox — HelixDB instance not startable | SCHEMA.md + check_schema.sh are text-only gates; D5 integration test is text-based (no running instance) |
| `crates/cli/src/main.rs` has no `lib.rs` — plumbing check expects `src/lib.rs` for strict crates only | CLI is advisory; plumbing check (A1) only verifies `[lints] workspace = true` in Cargo.toml, not lib.rs presence |
| Removing `thesix` from `crates/cli/Cargo.toml` may break import compilation | Verify `cargo check -p prv-cli` passes after removal; CLI doesn't use thesix in source |
| `tokio` and `dashmap` not yet workspace dependencies — needed by `crates/cache` | Add in C11; create `crates/cache` in Stage C before Stage D (which depends on cache for test_schema.rs) |

## Validation Criteria (exit conditions)

1. `python3 scripts/tetanus-check.py` passes with **0 errors** (warnings acceptable for density metrics)
2. `cargo fmt --all -- --check` passes
3. `cargo clippy --workspace --all-targets` passes (workspace `all = deny`; pedantic/nursery are warn-track)
4. `cargo test --workspace` passes (all existing + new tests)
5. `cargo deny check` passes (supply-chain gate)
6. `scripts/check_schema.sh` passes (SCHEMA.md declares all PRV labels + edges)
7. `prv-cli spec validate` passes (includes tetanus.toml existence check)
8. `prv-cli living update` generates all 4 `docs/*.md` files from `living.toml` projections (no hardcoded strings)
9. `prv-cli session handover` updates `living.toml` and regenerates docs
10. `crates/cache` crate compiles and its `NsCache::new()` builds six in-memory tiers
11. `crates/data` depends on `prv-cache` and `DataLoader::load_historical_cached()` compiles
12. `specs/parallelism.toml` exists with `status = "deferred"` and correct trigger conditions
13. `.github/workflows/ci.yml` has jobs: tetanus-check, rust-check (fmt+clippy+test+schema+deny+machete)
14. `AGENTS.md` updated with TETANUS quality gates section
15. `scripts/check_schema.sh` and `crates/cache/tests/test_schema.rs` both pass (D2, D5)
