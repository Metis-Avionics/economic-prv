# prv-monte-carlo

Monte Carlo shock simulation and probability estimation for the `prv` workspace.

## Overview

`prv-monte-carlo` provides stochastic simulation capabilities for economic stress testing and probabilistic forecasting:

- `Simulator` — Monte Carlo simulation driver with deterministic seeding
- `MonteCarloResults` — aggregated simulation outcomes (mean, median, quantiles, probabilities)
- `MonteCarloProbabilities` — probability estimates across scenarios
- `ShockSpec` / `ShockType` — shock definition and classification

The crate integrates with `prv-core` for state management and uses Cholesky decomposition with eigenvalue-clipping fallback for sampling from the state covariance.

## Shock Types

| Variant | Description |
|---------|-------------|
| `Demand` | Aggregate demand shock |
| `Supply` | Supply-side shock |
| `Investment` | Investment flow shock |
| `Geopolitical` | Geopolitical tension shock |
| `Fiscal` | Fiscal policy shock |
| `Migration` | Migration pressure shock |
| `Housing` | Housing market shock |
| `Financial` | Financial conditions shock |

## Usage

```rust
use nalgebra::SMatrix;
use prv_core::State;
use prv_monte_carlo::{Simulator, ShockSpec, ShockType};

let simulator = Simulator::new(42);
let mean = State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
let covariance = SMatrix::<f64, 8, 8>::identity() * 0.1;
let shocks = vec![
    ShockSpec { shock_type: ShockType::Demand, amplitude: 0.05, persistence: 0.8, autocorrelation: 0.1 },
    ShockSpec { shock_type: ShockType::Geopolitical, amplitude: 0.03, persistence: 0.9, autocorrelation: 0.05 },
];

let results = simulator.simulate(&mean, &covariance, 1000, 12, &shocks)?;
println!("Mean state: {:?}", results.mean.as_vector());
```

## Sampling Method

- Default: multivariate normal via `Cholesky` decomposition
- Fallback: nearest PSD via eigenvalue clipping (`1e-10` floor) when Cholesky fails

## Dependencies

- `prv-core` — core state types
- `prv-filter` — EKF estimation
- `nalgebra` — numerical arrays
- `rand` / `rand_distr` — random sampling
- `serde` / `thiserror` — serialization, errors

## License

MIT © Metis Avionics
