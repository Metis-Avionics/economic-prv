# prv-filter

Extended Kalman Filter (EKF) and observation/transition models for the `prv` workspace.

## Overview

`prv-filter` implements state estimation via the Extended Kalman Filter (EKF), providing:

- `Ekf` — generic EKF estimator with configurable state dimension
- `ObservationModel` — measurement model interface
- `TransitionModel` — state transition model interface

The crate bridges `prv-core` domain types with numerical linear algebra via `nalgebra`.

## Dependencies

- `prv-core` — core state and observation types
- `nalgebra` — linear algebra
- `tracing` — structured logging
- `thiserror` — error definitions
- `serde` — serialization

## License

MIT © Metis Avionics
