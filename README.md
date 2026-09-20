# prv

Research-grade economic simulation with EKF, Monte Carlo, and Spec-and-Go methodology.

## Overview

`prv` is a Rust workspace providing a comprehensive framework for economic simulation, state estimation, and policy evaluation. The project combines Extended Kalman Filter (EKF) state estimation, Monte Carlo shock simulation, and a structured policy engine within a Spec-and-Go development methodology.

## Workspace Crates

| Crate | Description |
|-------|-------------|
| `prv-core` | Core domain types: `State`, `Regime`, `Observation`, `TimeSeries`, `PressureReleaseValve` |
| `prv-filter` | Extended Kalman Filter (EKF) with configurable transition and observation models |
| `prv-monte-carlo` | Monte Carlo shock simulation via Cholesky decomposition with eigenvalue fallback |
| `prv-policy` | Stochastic policy engine producing `PolicyDistribution` outputs |
| `prv-geometry` | Quaternion-based state geometry and baseline comparison |
| `prv-data` | CSV/JSON ingestion, `DataFrame` abstraction, preprocessing, and observation conversion |
| `prv-evaluation` | Backtesting, metrics (RMSE, MAE, Brier, log-loss), and baseline comparison |
| `prv-cli` | Spec-and-Go CLI (`spec validate`, `living update`, `session handover`, `status`) |

## Quick Start

```bash
# Build everything
cargo build --all

# Run the full pipeline demo with faux data
cargo run --example run_pipeline -p prv-cli

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets
```

## Pipeline Demo

The workspace ships with `examples/faux_data.csv` (20 quarterly observations across 13 series) and `crates/cli/examples/run_pipeline.rs`, which wires the full stack:

```
DataLoader::load_historical  →  DataFrame
DataLoader::to_observations  →  Vec<Observation<10>>
Ekf::predict / update        →  State estimate
Simulator::simulate          →  MonteCarloResults
PolicyEngine::evaluate       →  PolicyDistribution
Evaluator::backtest          →  EvaluationResults
```

## Requirements

- Rust 1.98+
- Cargo
- Optional: Docker/Podman for HelixDB instance (for Spec-and-Go tooling)

## License

MIT © Metis Avionics
