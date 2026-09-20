# Plan: Spec-and-Go Economic PRV — Research-Grade Economic Simulation

## Context

The project at `/home/leo/prv/` is currently a blank HelixDB project (empty `Cargo.toml` with `name = "prv"`, empty `src/main.rs`). It must be transformed into a Rust workspace implementing a latent-state economic capacity model with EKF state estimation, Monte Carlo simulation, stochastic policy evaluation, quaternion-based regime geometry, historical data loading, and a backtesting/evaluation framework — all governed by the **Spec-and-Go** methodology where TOML specs are the source of truth and Markdown is a generated projection.

### Verified Dependencies

| Dependency | crates.io crate | Version | Role | Resolution |
|---|---|---|---|---|
| theSix | `thesix` | 0.2.3 | data-access-and-cache-abstraction | crates.io, features `redis`/`sled` |
| theMQL | `themql-runtime` + `themql-core` | 0.1.0 | message-query-and-dispatch | crates.io, feature `desktop` |

All dependencies are confirmed published on crates.io. No git-fallback needed. The spec.toml `[dependencies]` sections must be updated to reflect verified crate names and versions.

### Inherited Project State

- `helix.toml` — HelixDB project config from `helix init prv`. Keep as-is; may be repurposed for storing simulation results via `helix-db = "3"` (also used by theMQL).
- `AGENTS.md` — HelixDB workflow instructions. Update to include PRV project context.
- `examples/request.json` — HelixDB query example. Retain; may add PRV example queries.
- `.gitignore` — already ignores `/target`, `.helix/`, `.env`, `*.log`. No changes needed.

### Numerical Stack (decided)

- `nalgebra = "0.35"` — all linear algebra (state vectors, covariance matrices, quaternions). Confirmed consistent with theMQL's state_estimation spec which uses `SVector<f64, N>` / `SMatrix<f64, N, M>`.
- `ndarray = "0.17"` — Monte Carlo path arrays.
- `rand = "0.9"` + `rand_distr = "0.9"` — sampling. Multivariate normal implemented via Cholesky decomposition (`nalgebra::Cholesky`) applied to standard normal draws — same approach as theMQL state estimation.
- `statrs = "0.17"` — statistical distribution functions (Brier, log-loss, etc.).
- `clap = { version = "4", features = ["derive"] }` — CLI.
- `toml_edit = "0.22"` — TOML read/modify/write for spec validation and living.toml generation.
- `serde` + `serde_json` — serialization throughout.
- `thiserror = "2"` — error types (consistent with both deps).
- `anyhow = "1"` — CLI error handling.
- `tracing` + `tracing-subscriber` — structured logging.
- `proptest = "1"` — property testing.
- `csv = "1.3"` — data loading.
- `chrono = "0.4"` — timestamps.

### Crate Dependency Graph

```
core (no internal deps)
  ├── filter  → core
  ├── monte_carlo → core, filter
  ├── geometry → core
  ├── data → core
  └── policy → core, monte_carlo, filter
evaluation → core, filter, monte_carlo, policy, data, geometry
cli → all
```

## Key Decisions

1. **Workspace**: Virtual workspace (`Cargo.toml` at root with `[workspace]`). Internal crates use `path = "crates/..."` + `workspace = true` for shared deps. Edition 2024 (matching theSix's Rust 1.98 requirement).

2. **State vector**: 8-dimensional using `nalgebra::SVector<f64, 8>` — matches `specs/state_model.toml` dimensions and theMQL's EKF pattern with fixed dimensions.

3. **Observation mapping**: The EKF's observation dimension is "dataset-dependent" per spec. Implementation uses a trait `ObservationModel` with `fn observe(&self, state: &State) -> Observation` so different datasets can provide different mappings.

4. **Jacobian computation**: Finite-difference approximation (central differences) — the spec says transition is "nonlinear" without specifying analytical Jacobians. Central differences provide good accuracy for the 8-dim state.

5. **Quaternion model**: Experimental. Uses `nalgebra::Quaternion<f64>` for unit quaternion representation. Must be compared against the linear state model (evaluation baseline) per spec invariant.

6. **Spec-and-Go loop**: Each agent turn loads `spec.toml` + relevant `specs/*.toml`, inspects `docs/session.md` + `docs/handover.md`, executes within spec boundaries, then runs `prv-cli living update` to regenerate docs. The `living.toml` at root holds structured session/handover state; Markdown files are generated projections.

7. **Quality gates**: Follow theSix's "TETANUS" pattern — `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, property tests, `cargo deny check` (license/security audit). `clippy::pedantic` + `clippy::nursery` enabled. `forbid(unsafe_code)` enforced.

8. **CLI `prv-cli`**: Single binary in `crates/cli/`. Uses `clap` derive API. The `living update` subcommand reads all `specs/*.toml` + `living.toml`, generates the four `docs/*.md` files. `spec validate` checks TOML validity, required sections, and unresolved dependencies.

9. **Existing HelixDB integration**: Retained but not central. `helix-db` used as the state store for simulation results via theSix cache and direct HelixDB queries. The `helix.toml` and `AGENTS.md` remain but should be updated to reference PRV commands alongside HelixDB commands.

## Task List

### Stage 1: Spec & Project Setup

- [ ] **Task 1**: Update `spec.toml` with resolved dependencies. Replace `[dependencies.theSix]` `crate_name = "UNRESOLVED"` with:
  ```toml
  [dependencies.theSix]
  required = true
  role = "data-access-and-cache-abstraction"
  crate_name = "thesix"
  version = "0.2.3"
  source = "crates-io"
  features = ["redis", "sled"]
  resolution_must_be_verified = true
  ```
  Replace `[dependencies.theMQL]` similarly:
  ```toml
  [dependencies.theMQL]
  required = true
  role = "message-query-and-dispatch"
  crate_name = "themql-runtime"
  version = "0.1.0"
  source = "crates-io"
  features = ["desktop"]
  companion_crates = ["themql-core"]
  resolution_must_be_verified = true
  ```

- [ ] **Task 2**: Create `specs/dependencies.toml` — records the full resolution chain:
  ```toml
  [theSix]
  crate_name = "thesix"
  version = "0.2.3"
  source = "crates-io"
  verified = "2026-09-20"
  features = ["redis", "sled"]
  role = "data-access-and-cache-abstraction"

  [theMQL]
  crate_name = "themql-runtime"
  version = "0.1.0"
  source = "crates-io"
  verified = "2026-09-20"
  features = ["desktop"]
  companion_crates = ["themql-core"]
  role = "message-query-and-dispatch"
  ```
  (This resolves the dependency rule: no unverified crate identifiers, no fake versions.)

- [ ] **Task 3**: Write all spec TOML files exactly as provided by the user, into `specs/`:
  - `specs/methodology.toml`
  - `specs/state_model.toml`
  - `specs/ekf.toml`
  - `specs/monte_carlo.toml`
  - `specs/policy_engine.toml`
  - `specs/quaternion_model.toml`
  - `specs/data.toml`
  - `specs/evaluation.toml`
  - `specs/cli.toml`

- [ ] **Task 4**: Create `living.toml` at root — structured state file that drives CLI and generates docs. Initial structure:
  ```toml
  [handover]
  status = "initial"
  phase = "setup"
  last_updated = "<timestamp>"
  summary = "Project initialized with Spec-and-Go methodology. Dependencies verified: thesix 0.2.3, themql-* 0.1.0."
  remaining = []
  decisions = []
  completed = []
  changed = []
  validated = []
  known_failures = []
  next_action = "Scaffold Rust workspace crates"
  open_questions = []

  [changelog]
  entries = [
    { type = "spec_created", description = "Initial spec.toml with verified dependencies", timestamp = "<timestamp>" },
    { type = "dependency_resolved", description = "theSix → thesix 0.2.3 (crates.io)", timestamp = "<timestamp>" },
    { type = "dependency_resolved", description = "theMQL → themql-runtime 0.1.0 + themql-core (crates.io)", timestamp = "<timestamp>" },
  ]
  ```

### Stage 2: Workspace & Directory Structure

- [ ] **Task 5**: Convert root `Cargo.toml` to virtual workspace:
  ```toml
  [workspace]
  resolver = "2"
  members = [
      "crates/core",
      "crates/filter",
      "crates/monte_carlo",
      "crates/policy",
      "crates/geometry",
      "crates/data",
      "crates/evaluation",
      "crates/cli",
  ]

  [workspace.package]
  version = "0.1.0"
  edition = "2024"
  license = "MIT"
  repository = "https://github.com/Metis-Avionics/economic-prv"
  authors = ["Metis Avionics"]

  [workspace.dependencies]
  nalgebra = "0.35"
  ndarray = "0.17"
  rand = "0.9"
  rand_distr = "0.9"
  statrs = "0.17"
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  thiserror = "2"
  anyhow = "1"
  clap = { version = "4", features = ["derive"] }
  toml_edit = "0.22"
  tracing = "0.1"
  tracing-subscriber = "0.3"
  proptest = "1"
  csv = "1.3"
  chrono = { version = "0.4", features = ["serde"] }
  thesix = { version = "0.2", features = ["redis", "sled"] }
  themql-core = "0.1"
  themql-runtime = { version = "0.1", features = ["desktop"] }
  ```
  Delete `src/main.rs` (replaced by workspace crates). Keep the old `[package]` info in `spec.toml` — it's already `[project]`.

- [ ] **Task 6**: Create directory structure:
  ```
  crates/core/src/
  crates/filter/src/
  crates/monte_carlo/src/
  crates/policy/src/
  crates/geometry/src/
  crates/data/src/
  crates/evaluation/src/
  crates/cli/src/
  docs/   (auto-generated, create .gitkeep or initial stubs)
  specs/
  ```

- [ ] **Task 7**: Scaffold each crate with its `Cargo.toml`:
  - `crates/core/Cargo.toml` — `[package]` with `name = "prv-core"`, `workspace = true`. Deps: `nalgebra`, `serde`, `serde_json`, `thiserror`, `chrono`.
  - `crates/filter/Cargo.toml` — `name = "prv-filter"`, depends on `prv-core`. Deps: `nalgebra`, `serde`, `thiserror`, `tracing`.
  - `crates/monte_carlo/Cargo.toml` — `name = "prv-monte-carlo"`, depends on `prv-core`, `prv-filter`. Deps: `nalgebra`, `ndarray`, `rand`, `rand_distr`, `serde`, `thiserror`, `tracing`.
  - `crates/policy/Cargo.toml` — `name = "prv-policy"`, depends on `prv-core`, `prv-monte-carlo`, `prv-filter`. Deps: `nalgebra`, `ndarray`, `serde`, `thiserror`, `tracing`.
  - `crates/geometry/Cargo.toml` — `name = "prv-geometry"`, depends on `prv-core`. Deps: `nalgebra`, `serde`, `thiserror`, `tracing`.
  - `crates/data/Cargo.toml` — `name = "prv-data"`, depends on `prv-core`. Deps: `csv`, `serde`, `serde_json`, `thiserror`, `chrono`, `tracing`.
  - `crates/evaluation/Cargo.toml` — `name = "prv-evaluation"`, depends on `prv-core`, `prv-filter`, `prv-monte-carlo`, `prv-policy`, `prv-data`, `prv-geometry`. Deps: `nalgebra`, `ndarray`, `serde`, `thiserror`, `statrs`, `tracing`.
  - `crates/cli/Cargo.toml` — `name = "prv-cli"`, `bin = [{ name = "prv-cli" }]`. Depends on all internal crates + external deps. Deps: `clap`, `toml_edit`, `serde`, `serde_json`, `anyhow`, `tracing`, `tracing-subscriber`, `thesix`, `themql-core`, `themql-runtime`.

- [ ] **Task 8**: Run `cargo check` to verify workspace compiles with stub `lib.rs` files (all return `Ok(())` or empty).

### Stage 3: Core Crate (`crates/core`)

- [ ] **Task 9**: Implement `crates/core/src/lib.rs` with:
  - Crate-level lints: `#![forbid(unsafe_code)]`, `#![warn(clippy::pedantic, clippy::nursery)]`
  - `mod state` — `State` struct: `pub struct State(pub SVector<f64, 8>)` with named accessors for each dimension (`capacity()`, `investment()`, `labour_absorption()`, `fiscal_capacity()`, `demand_pressure()`, `housing_pressure()`, `geopolitical_load()`, `migration_pressure()`). Derive `Serialize`, `Deserialize`, `Clone`, `Debug`.
  - `mod observation` — `Observation` struct: `pub struct Observation(pub SVector<f64, D>)` where D is dynamic (dataset-dependent). Named accessors for the 10 required series.
  - `mod regime` — `Regime` enum: `Expansion`, `Saturation`, `Contraction`, `Recovery`, `StructuralShock`. With `from_pressure()` method classifying regime from state vector.
  - `mod prv` — `PressureReleaseValve` struct: holds capacity, demand, housing, fiscal, geopolitical sub-pressures. `fn value(&self, state: &State) -> f64`.
  - `mod time` — `TimeSeries<T>` struct: timestamp + value, with `interpolate()`, `resample()` for quarterly data.
  - `mod error` — `PrvError` enum using `thiserror`.

### Stage 4: Filter Crate (`crates/filter`)

- [ ] **Task 10**: Implement EKF per `specs/ekf.toml`:
  - `Ekf` struct: `x_hat: SVector<f64, 8>`, `P: SMatrix<f64, 8, 8>`, `Q: SMatrix<f64, 8, 8>`, `R: SMatrix<f64, D, D>`.
  - `trait TransitionModel`: `fn f(&self, state: &State, control: Option<&Control>) -> State` — nonlinear transition.
  - `trait ObservationModel`: `fn h(&self, state: &State) -> Observation` — nonlinear observation mapping.
  - `fn jacobian_f(&self, state: &State, dt: f64) -> SMatrix<f64, 8, 8>` — central difference approximation of ∂f/∂x.
  - `fn jacobian_h(&self, state: &State) -> SMatrix<f64, D, 8>` — central difference approximation of ∂h/∂x.
  - `fn predict(&mut self, u: Option<&Control>)` — prediction step per spec equations.
  - `fn update(&mut self, z: &Observation)` — update step with Kalman gain, innovation.
  - `fn stabilize_covariance(&mut self)` — enforce symmetric + positive semidefinite (eigenvalue clamping).
  - Validation per `specs/ekf.toml[validation.required]`: `innovation_statistics()`, `covariance_symmetry()`, `finite_state()`, `finite_covariance()`.

### Stage 5: Monte Carlo Crate (`crates/monte_carlo`)

- [ ] **Task 11**: Implement Monte Carlo simulation per `specs/monte_carlo.toml`:
  - `ShockType` enum: Demand, Supply, Investment, Geopolitical, Fiscal, Migration, Housing, Financial.
  - `ShockSpec` struct: amplitude, persistence, autocorrelation per `specs/data.toml[geopolitical]` and state_model pressure fields.
  - `fn sample_state(&self, ekf: &EKF, n: usize) -> Vec<State>` — sample from multivariate normal using filtered state mean + covariance (Cholesky factorization via nalgebra).
  - `fn run_path(&self, initial: &State, horizon: usize, shocks: Vec<ShockSpec>) -> Vec<State>` — simulate one trajectory.
  - `fn simulate(&self, ekf: &EKF, n_paths: usize, horizon: usize) -> MonteCarloResults`.
  - `MonteCarloResults` struct with: `mean`, `median`, `quantiles` ([0.01, 0.05, 0.10, 0.50, 0.90, 0.95, 0.99]), `probabilities` (capacity_increase/decline, investment_increase/decline, regime_transition), `tail_risk`, `regime_distribution`.
  - Deterministic seed via `StdRng::seed_from_u64()` — record seed + sample count per validation spec.

### Stage 6: Policy Crate (`crates/policy`)

- [ ] **Task 12**: Implement stochastic policy engine per `specs/policy_engine.toml`:
  - `PolicyInstrument` enum: InterestRate, QuantitativeEasing, SovereignWealthFundDeployment, FiscalSpending, Taxation, InfrastructureInvestment, MigrationCapacity.
  - `PolicyWeights` struct: capacity, investment, employment, inflation, debt, housing_pressure, systemic_risk (from spec).
  - `fn evaluate(&self, mc: &MonteCarloResults, regime: &Regime) -> PolicyDistribution` — evaluates policy intervention against state distribution, returns probabilistic action distribution (no single point policy).
  - `fn regime_response(&self, regime: &Regime) -> PolicyBias` — matches spec regime_response table.
  - Constraint checking: `quantitative_easing` (inflation_and_financial_stability), `sovereign_wealth_fund` (fund_liquidity_and_intergenerational_equity).
  - Output type: `PolicyDistribution` with `recommended_action_distribution`, `expected_state_change`, `downside_distribution`, `tail_risk`, `constraint_violations`.

### Stage 7: Geometry Crate (`crates/geometry`)

- [ ] **Task 13**: Implement quaternion model per `specs/quaternion_model.toml`:
  - `QuaternionState` struct: 4D reduced state (capacity, investment, demand, pressure) encoded as `UnitQuaternion<f64>`.
  - `fn to_quaternion(&self, state: &State) -> UnitQuaternion<f64>` — map 8-dim state to 4D quaternion. (Capacity/investment → x/y, demand → z, pressure → w alignment.)
  - `fn transition(&self, q_prev: &UnitQuaternion<f64>, q_curr: &UnitQuaternion<f64>) -> UnitQuaternion<f64>` — `q_delta = inverse(q_prev) * q_curr`.
  - `fn angle_of_rotation(&self, q_delta: &UnitQuaternion<f64>) -> f64` — extract regime transition magnitude.
  - Constraints: unit norm enforced, zero quaternion rejected.
  - `fn baseline_compare(&self, q_results: &MonteCarloResults, linear_results: &MonteCarloResults) -> Comparison` — required invariant: quaternion model must be compared against non-quaternion baseline.

### Stage 8: Data Crate (`crates/data`)

- [ ] **Task 14**: Implement data loader per `specs/data.toml`:
  - `fn load_historical(&self, path: &str) -> Result<DataFrame>` — CSV/JSON with source attribution.
  - `TimeSeries` for each of the 10 required series + 3 geopolitical series.
  - `fn preprocess(&self, data: &mut DataFrame)` — normalization, outlier marking, missing value handling (all explicit per spec).
  - `fn to_observations(&self, data: &DataFrame) -> Vec<Observation>` — construct observation vectors from raw series.

### Stage 9: Evaluation Crate (`crates/evaluation`)

- [ ] **Task 15**: Implement evaluation framework per `specs/evaluation.toml`:
  - `Baseline` enum: Naive, MovingAverage, LinearStateModel, NonQuaternionStateModel.
  - `fn backtest(&self, model: &impl Model, data: &TimeSeries, window: usize) -> EvaluationResults` — rolling window, out-of-sample.
  - Metrics: `calibration_score()`, `brier_score()`, `log_loss()`, `rmse()`, `mae()`, `regime_detection_accuracy()`, `false_transition_rate()`, `tail_risk_error()`.
  - EKF validation: `innovation_consistency()`, `covariance_stability()`, `state_bound_check()`.
  - Monte Carlo validation: `seed_reproducibility()`, `distribution_stability()`, `sample_convergence()`.
  - Policy validation: `counterfactual_integrity()`, `constraint_integrity()`, `policy_sensitivity()`, `shock_sensitivity()`.
  - Research integrity invariants enforced as compile-time tests: `simulation_not_equal_to_forecast`, `correlation_not_equal_to_causation`.

### Stage 10: CLI Crate (`crates/cli`)

- [ ] **Task 16**: Implement `prv-cli` binary per `specs/cli.toml`:
  - `Command` enum with `clap` derive: `Start`, `Validate`, `Plan`, `Living`, `Handover`, `Status`.
  - `prv-cli spec start` — loads `spec.toml` + all `specs/*.toml` + `docs/session.md` + `docs/handover.md`. Validates spec integrity. Prints current project state.
  - `prv-cli spec validate` — checks: valid TOML, required sections present, dependencies resolved (fails on `UNRESOLVED`), all spec files present. Exits non-zero on failure.
  - `prv-cli spec plan` — enters planning mode. Loads current specs, opens a session for spec extension.
  - `prv-cli living update` — reads all `specs/*.toml` + `living.toml`, generates:
    - `docs/living.md` — project overview, architecture, dependencies (projection of spec.toml)
    - `docs/changelog.md` — spec changes, architecture changes, model changes (from living.toml changelog)
    - `docs/session.md` — current turn phase, loaded specs, actions taken
    - `docs/handover.md` — completed, changed, validated, known_failures, next_action, open_questions
  - `prv-cli session handover` — writes structured handover to `living.toml` + regenerates `docs/handover.md`.
  - `prv-cli status` — shows workspace status, crate tree (`cargo tree`), spec validation status, last handover.

- [ ] **Task 17**: Create `AGENTS.md` update — add PRV project context section:
  ```markdown
  ## PRV Project Context

  This project uses the Spec-and-Go methodology. Each agent turn:
  1. Loads `spec.toml` + `specs/*.toml` as the authoritative contract
  2. Inspects `docs/session.md` and `docs/handover.md` for context
  3. Executes within current spec boundaries
  4. Runs `prv-cli living update` at turn completion
  5. Records handover for the next agent

  The TOML spec is the memory; Markdown is a generated projection.
  ```

### Stage 11: Documentation Pipeline

- [ ] **Task 18**: Create initial `docs/` stubs (will be overwritten by `prv-cli living update`):
  - `docs/living.md` — generated from spec.toml
  - `docs/changelog.md` — generated from living.toml changelog
  - `docs/session.md` — generated from current session state
  - `docs/handover.md` — generated from living.toml handover

- [ ] **Task 19**: Verify the Spec-and-Go flow end-to-end:
  - Run `cargo run -p prv-cli -- spec validate` — should pass
  - Run `cargo run -p prv-cli -- living update` — should generate all docs/
  - Run `cargo run -p prv-cli -- status` — should show workspace state

### Stage 12: Validation & Testing

- [ ] **Task 20**: Run full quality gates:
  - `cargo fmt --check` — formatting
  - `cargo clippy --workspace -- -D warnings` — lint (pedantic + nursery)
  - `cargo test --workspace` — unit + integration tests
  - `cargo deny check` — license/security (if deny is installed; otherwise note as future)

- [ ] **Task 21**: Verify spec invariants:
  - `spec.toml` dependencies show `crate_name` = verified values (not `UNRESOLVED`)
  - `specs/dependencies.toml` records verification timestamps
  - `specs/quaternion_model.toml[validation.baseline_model_required] = true` is enforced by a test in `crates/evaluation`
  - `specs/evaluation.toml[research_integrity]` invariants are enforced by tests

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| EKF numerical instability with 8×8 covariance | Symmetric clamping + eigenvalue floor; validate per `specs/ekf.toml[validation]` |
| Multivariate normal sampling fails on non-PSD covariance | Cholesky fallback to SVD or nearest-PSD projection in `crates/monte_carlo` |
| Quaternion model doesn't beat baseline | Required by spec invariant; evaluation crate has explicit comparison test |
| theMQL/theSix API incompatibility | Verify against crate docs during implementation; `cargo check` early |
| Cargo workspace edition 2024 requires Rust 1.98+ | theSix requires 1.98; check with `rustc --version` early; rustup if needed |
| Spec drift (markdown ≠ spec) | CLI generates markdown from TOML only; no manual edits per `specs/cli.toml[validation]` policy |

## Validation Criteria (exit conditions for implementation agent)

1. `cargo check --workspace` passes with zero warnings
2. `cargo clippy --workspace -- -D warnings` passes
3. `cargo test --workspace` passes (unit + integration tests)
4. `cargo run -p prv-cli -- spec validate` passes (all deps resolved, no `UNRESOLVED`)
5. `cargo run -p prv-cli -- living update` generates all 4 `docs/*.md` files
6. `specs/dependencies.toml` has `verified` timestamps for both deps
7. `specs/quaternion_model.toml` invariant (baseline comparison) enforced by test
8. `specs/evaluation.toml` research integrity invariants enforced by tests
9. EKF produces finite, symmetric, positive-semidefinite covariance
10. Monte Carlo with same seed produces identical results (reproducibility)
