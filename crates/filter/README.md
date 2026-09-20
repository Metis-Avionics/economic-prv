# prv-filter

Extended Kalman Filter (EKF) and observation/transition models for the `prv` workspace.

## Overview

`prv-filter` implements state estimation via the Extended Kalman Filter (EKF), providing:

- `Ekf<M, O>` — generic EKF estimator with configurable state dimension (8)
- `TransitionModel` — state transition model interface (`f(x, u)`)
- `ObservationModel<D>` — measurement model interface (`h(x)`, Jacobian, noise covariance `R`)

The crate bridges `prv-core` domain types with numerical linear algebra via `nalgebra`.

## Usage

```rust
use nalgebra::SMatrix;
use prv_core::State;
use prv_filter::{Ekf, DefaultTransition, ObservationModel};

struct MyModel;

impl ObservationModel<4> for MyModel {
    fn h(&self, state: &State) -> prv_core::Observation<4> {
        // linear observation of first 4 state dimensions
        let h = SMatrix::<f64, 4, 8>::identity();
        let vec = &h * state.as_vector();
        prv_core::Observation::new(vec.into())
    }

    fn jacobian_h(&self, _state: &State) -> SMatrix<f64, 4, 8> {
        SMatrix::<f64, 4, 8>::identity()
    }

    fn r(&self) -> SMatrix<f64, 4, 4> {
        SMatrix::<f64, 4, 4>::identity() * 0.1
    }
}

let mut ekf = Ekf::new(
    DefaultTransition,
    MyModel,
    State::default(),
    SMatrix::<f64, 8, 8>::identity() * 0.5,
    SMatrix::<f64, 8, 8>::identity() * 0.01,
    SMatrix::<f64, 4, 4>::identity() * 0.1,
);

ekf.predict(None)?;
ekf.update(&observation)?;
```

## Validation

The EKF exposes validation helpers:
- `innovation_statistics` — NIS and std dev
- `covariance_symmetry` — P ≈ Pᵀ check
- `covariance_psd` — positive semidefinite check
- `finite_state` / `finite_covariance` — non-finite guard checks

## Dependencies

- `prv-core` — core state and observation types
- `nalgebra` — linear algebra
- `thiserror` — error definitions
- `serde` — serialization

## License

MIT © Metis Avionics
