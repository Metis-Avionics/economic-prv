# prv-evaluation

Backtesting, metrics, and model evaluation for the `prv` workspace.

## Overview

`prv-evaluation` provides quantitative evaluation of simulation and estimation pipelines:

- `Evaluator` — orchestrates backtests across models and baselines
- `Model` — candidate model trait
- `Baseline` — reference model definitions (`Naive`, `MovingAverage`, `LinearStateModel`, `NonQuaternionStateModel`)
- `MetricResults` — computed evaluation metrics
- `backtest` — historical replay and scoring logic

The crate depends on data loading (`prv-data`), EKF estimates (`prv-filter`), Monte Carlo results (`prv-monte-carlo`), and policy outputs (`prv-policy`).

## Metrics

| Metric | Description |
|--------|-------------|
| `rmse` | Root mean squared error |
| `mae` | Mean absolute error |
| `calibration_score` | Probabilistic calibration |
| `brier_score` | Brier score for probabilistic forecasts |
| `log_loss` | Logarithmic loss |
| `regime_detection_accuracy` | Regime classification accuracy |
| `false_transition_rate` | Rate of spurious regime transitions |
| `tail_risk_error` | Tail risk estimation error |

## Baselines

| Baseline | Description |
|----------|-------------|
| `Naive` | Random walk (no change) |
| `MovingAverage` | Historical mean |
| `LinearStateModel` | Linear state transition |
| `NonQuaternionStateModel` | Non-quaternion geometric baseline |

## Usage

```rust
use prv_evaluation::{Evaluator, Model, Baseline};

struct MyModel;
impl Model for MyModel {
    fn predict(&self, state: &prv_core::State) -> prv_core::State {
        state.clone()
    }
}

let evaluator = Evaluator::new();
let results = evaluator.backtest(&MyModel, &time_series, window_size);

println!("RMSE: {}", results.metrics.rmse);
println!("MAE: {}", results.metrics.mae);

for (baseline, metrics) in results.baseline_comparison {
    println!("{baseline:?} RMSE: {}", metrics.rmse);
}
```

## Research Integrity

Compile-time invariants enforced:
- `simulation_not_equal_to_forecast`
- `correlation_not_equal_to_causation`

## Dependencies

- `prv-core` — state and time-series types
- `prv-filter` — EKF estimates
- `prv-monte-carlo` — simulation results
- `prv-policy` — policy distributions
- `prv-data` — data frames
- `prv-geometry` — geometric state comparisons
- `nalgebra` / `ndarray` / `statrs` — numerical and statistical types
- `serde` / `thiserror` — serialization, errors

## License

MIT © Metis Avionics
