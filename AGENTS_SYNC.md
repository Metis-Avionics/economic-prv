# AGENTS_SYNC.md — TCAS Coordination Protocol

This file coordinates multiple Kilo agents working in the same workspace.
Treat it as a lightweight TCAS (Terminal Collision Avoidance System): each
agent announces its flight plan before editing, and conflicts are resolved
by explicit handoff rather than merge conflicts.

## Active Sessions

| Agent | Session ID | Working Directory | Started | Last Update | Status |
|---|---|---|---|---|---|
| Session A (consolidated) | prv-impl | /home/leo/prv | 2026-09-20T21:10:00+01:00 | 2026-09-20T22:41:00+01:00 | active |
| Session B (sibling) | prv-impl | /home/leo/prv | 2026-09-20T21:32:00+01:00 | 2026-09-20T22:40:00+01:00 | merged-into-A |

## Flight Plans

Each agent declares its current and planned work here before touching files.

### Session A — Consolidated Workstream (TETANUS + Cache + CI + Living)

**Completed work:**
- A1-A11: TETANUS enforcement infrastructure
  - `scripts/tetanus-check.py` (copied from NS-Pro, `check_plumbing()` uses dynamic workspace member discovery)
  - `clippy.toml` (thresholds)
  - `tetanus.toml` (PRV-specific scope)
  - Root `Cargo.toml` updated with `[workspace.lints.rust]`, `[workspace.lints.clippy]`, `crates/cache` member, `tokio` and `dashmap` workspace deps
  - All 8 existing crate `Cargo.toml` files updated with `[lints] workspace = true`
  - Strict crate lib.rs headers: removed `#![warn(clippy::pedantic, clippy::nursery)]`, added `#![deny(clippy::unwrap_used, clippy::expect_used)]` and debug-macro denies
  - Advisory crate lib.rs headers and `cli/src/main.rs` cleaned
- A8: Strict-crate remediations
  - `crates/monte_carlo/src/simulator.rs`: `sample_state` returns `Result<Vec<State>, PrvError>`; Cholesky uses `ok_or_else`/`ok_or` instead of `expect`; `simulate` split into `prepare_cholesky`, `run_paths`, `run_single_path`, `compute_results`, `compute_mean`, `compute_median`, `compute_quantiles`, `compute_regime_stats`, `compute_tail_risk`; `.unwrap()` on `p.last().cloned()` replaced with `.filter_map(|p| p.last().cloned())`
  - `crates/policy/src/engine.rs`: `sample_noise` uses `r.sample::<f64, _>(StandardNormal) * 0.1` instead of `Normal::new(0.0, 0.1).unwrap()`; `evaluate` split into `compute_state_signals`, `score_monetary_policy`, `score_fiscal_policy`, `score_real_economy`, `build_policy_distribution`, `extract_scores`, `compute_expected`, `compute_downside`, `check_constraints`; removed `#[allow(clippy::too_many_lines)]`
  - Added `use rand::RngExt` to `engine.rs` for `.sample()` method
  - Added `use prv_core::PrvError` to `simulator.rs`
  - Moved `StateSignals`, `InstrumentScores` structs and `sample_noise` function outside `impl PolicyEngine` block
  - Added `debug_assert!` in `stabilize_covariance` and `assert!` for finiteness
- B1-B2: Supply-chain gates
  - `deny.toml` with advisory exceptions for `RUSTSEC-2024-0384` (instant/sled) and `RUSTSEC-2025-0057` (fxhash/sled)
  - `.cargo/audit.toml` mirroring advisory exceptions
- C1-C11: thesix cache integration (merged from Session B)
  - Created `crates/cache` crate with `thesix` integration
  - Created `crates/cache/src/{lib.rs,manager.rs,keys.rs,stats.rs,index.rs}`
  - `prv-data` depends on `prv-cache`; `DataLoader::load_historical_cached()` added
  - `prv-cli` depends on `prv-cache` via `CliCache`
- D1-D5: HelixDB schema + CI gate
  - `SCHEMA.md` with PRV labels/edges
  - `scripts/check_schema.sh` validates labels/edges
  - `examples/helix-query.json` with PRV query example
  - `.github/workflows/ci.yml` with all quality gates (tetanus, fmt, clippy, test, schema, deny, machete)
- E1-E7: CLI living update / session handover
  - `validate_spec`, `update_living`, `session_handover`, `print_status` wired through cache
  - `thesix` added back to `prv-cli/Cargo.toml` for `CliCache`
- H1: AGENTS.md updated with Quality Gates section
- I1-I2: `living.toml` updated with remaining work and changelog entries

**Validation status:**
- `cargo check --workspace` passes
- `cargo fmt --all -- --check` passes
- `cargo clippy --workspace --all-targets` passes (0 errors; pedantic/nursery warnings remain but are on warn track)
- `cargo test --workspace` passes (72 tests)
- `python3 scripts/tetanus-check.py` passes (0 errors, 16 warnings)
- `bash scripts/check_schema.sh` passes

**Next actions:**
- Stage F: Add `specs/parallelism.toml` for Apalis/Rayon optimization targets
- Stage D remainder: Add `specs/helixdb.toml` and live HelixDB integration tests
- Consider reducing TETANUS assertion density warnings by adding more `debug_assert!`/guards in hot paths

**Files owned by Session A (consolidated):**
- `/home/leo/prv/scripts/tetanus-check.py`
- `/home/leo/prv/clippy.toml`
- `/home/leo/prv/tetanus.toml`
- `/home/leo/prv/Cargo.toml`
- `/home/leo/prv/deny.toml`
- `/home/leo/prv/.cargo/audit.toml`
- `/home/leo/prv/SCHEMA.md`
- `/home/leo/prv/scripts/check_schema.sh`
- `/home/leo/prv/examples/helix-query.json`
- `/home/leo/prv/.github/workflows/ci.yml`
- `/home/leo/prv/AGENTS.md`
- `/home/leo/prv/living.toml`
- `/home/leo/prv/spec.toml`
- `/home/leo/prv/specs/cli.toml`
- All crate `Cargo.toml` files
- All strict-crate `src/lib.rs` headers
- `crates/monte_carlo/src/simulator.rs`
- `crates/policy/src/engine.rs`
- `crates/filter/src/ekf.rs`
- `crates/data/src/cache.rs` (inline, superseded by prv-cache)
- `crates/cli/src/cache.rs` (inline, superseded by prv-cache)
- `crates/data/src/loader.rs`
- `crates/cli/src/main.rs`
- `crates/cache/Cargo.toml` (NEW)
- `crates/cache/src/{lib.rs,manager.rs,keys.rs,stats.rs,index.rs}` (NEW)

## Session B — Merged into Session A

**Status:** Completed and folded into Session A's workstream.

**Work absorbed:**
- `crates/cache` crate created with `thesix` integration
- `NsCache`, `CacheStats`, `KeyIndex` implementations
- `DataLoader::load_historical_cached()` integration
- All cache-related files now under Session A ownership

**Original flight plan (completed):**
- C1-C4: Cache crate scaffolding
- C9: DataLoader integration
- All files listed in Session B's "Files touched" section are now owned by Session A

## Coordination Rules

1. **Announce before edit:** Before editing any file listed in another agent's
   "Files touched" section, send a brief message to that agent via the shared
   terminal or coordination channel.

2. **Non-overlapping ownership:** If both agents need the same file, the agent
   that touched it first "owns" it for the current task. The other agent must
   wait or work on a different file.

3. **Sequential Cargo.toml edits:** Only one agent should edit any crate's
   `Cargo.toml` at a time. If Session B needs to update `crates/data/Cargo.toml`
   after Session A already added `[lints] workspace = true`, Session B should
   append the `prv-cache` dependency, not replace the file.

4. **Git as source of truth:** After significant changes, each agent should
   commit its work with a clear message (e.g., `feat(tetanus): add clippy.toml and workspace lints`).
   This prevents lost work and makes conflicts resolvable.

5. **Handoff protocol:** When an agent completes its task, it should:
   - Update `AGENTS_SYNC.md` with its completed work and new "Files touched" list.
   - Clear its "Current work" section.
   - Set its status to `idle` or `completed`.

6. **Emergency brake:** If both agents edit the same file simultaneously and
   `git diff` shows overlapping hunks, STOP. Do not force-push. Coordinate
   via the shared terminal, then re-apply changes sequentially.
