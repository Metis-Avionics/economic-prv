# prv

Research-grade economic simulation with EKF, Monte Carlo, and Spec-and-Go methodology.

## Overview

`prv` is a Rust workspace providing a comprehensive framework for economic simulation, state estimation, and policy evaluation. The project combines Extended Kalman Filter (EKF) state estimation, Monte Carlo shock simulation, and a structured policy engine within a Spec-and-Go development methodology.

## Workspace Crates

| Crate | Description |
|-------|-------------|
| `prv-core` | Core domain types, state, regime, observation, and time-series abstractions |
| `prv-filter` | Extended Kalman Filter (EKF) and observation/transition models |
| `prv-monte-carlo` | Monte Carlo shock simulation and probability estimation |
| `prv-policy` | Policy engine, instrument weights, and bias distributions |
| `prv-geometry` | Quaternion state geometry and comparison utilities |
| `prv-data` | CSV/JSON data loading and DataFrame abstractions |
| `prv-evaluation` | Backtesting, metrics, and model evaluation |
| `prv-cli` | Command-line interface for Spec-and-Go workflow |

## Requirements

- Rust 1.98+
- Cargo
- Optional: Docker/Podman for HelixDB instance (for Spec-and-Go tooling)

## Building

```bash
cargo build --all
```

## Testing

```bash
cargo test --all
```

## License

MIT © Metis Avionics
