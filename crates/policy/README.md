# prv-policy

Policy engine, instrument weights, and bias distributions for the `prv` workspace.

## Overview

`prv-policy` implements the policy layer of the economic simulation framework, translating model outputs into actionable policy instruments:

- `PolicyEngine` — main policy decision engine
- `PolicyInstrument` — individual policy tool definition
- `PolicyWeights` — weighting across policy instruments
- `PolicyDistribution` — distribution over policy actions
- `PolicyBias` — bias correction for policy outputs

The engine consumes `MonteCarloResults` and `Regime` classifications from upstream crates.

## Dependencies

- `prv-core` — regime and state types
- `prv-monte-carlo` — simulation results
- `prv-filter` — EKF state estimates
- `nalgebra` / `ndarray` — numerical types
- `serde` / `thiserror` / `tracing` — serialization, errors, logging

## License

MIT © Metis Avionics
