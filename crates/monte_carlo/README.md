# prv-monte-carlo

Monte Carlo shock simulation and probability estimation for the `prv` workspace.

## Overview

`prv-monte-carlo` provides stochastic simulation capabilities for economic stress testing and probabilistic forecasting:

- `Simulator` — Monte Carlo simulation driver
- `MonteCarloResults` — aggregated simulation outcomes
- `MonteCarloProbabilities` — probability estimates across scenarios
- `ShockSpec` / `ShockType` — shock definition and classification

The crate integrates with `prv-core` for state management and `prv-filter` for EKF-based estimation within simulated trajectories.

## Dependencies

- `prv-core` — core state types
- `prv-filter` — EKF estimation
- `nalgebra` / `ndarray` — numerical arrays
- `rand` / `rand_distr` — random sampling
- `serde` / `thiserror` / `tracing` — serialization, errors, logging

## License

MIT © Metis Avionics
