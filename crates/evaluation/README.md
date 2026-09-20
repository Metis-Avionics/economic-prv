# prv-evaluation

Backtesting, metrics, and model evaluation for the `prv` workspace.

## Overview

`prv-evaluation` provides quantitative evaluation of simulation and estimation pipelines:

- `Evaluator` — orchestrates backtests across models and baselines
- `Model` / `Baseline` — candidate and reference model definitions
- `MetricResults` — computed evaluation metrics
- `backtest` — historical replay and scoring logic

The crate depends on data loading (`prv-data`), EKF estimates (`prv-filter`), Monte Carlo results (`prv-monte-carlo`), and policy outputs (`prv-policy`).

## Dependencies

- `prv-core` — state and time-series types
- `prv-filter` — EKF estimates
- `prv-monte-carlo` — simulation results
- `prv-policy` — policy distributions
- `prv-data` — data frames
- `prv-geometry` — geometric state comparisons
- `nalgebra` / `ndarray` / `statrs` — numerical and statistical types
- `serde` / `thiserror` / `tracing` — serialization, errors, logging

## License

MIT © Metis Avionics
