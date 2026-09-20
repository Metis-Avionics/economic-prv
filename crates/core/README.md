# prv-core

Core domain types, state, regime, observation, and time-series abstractions for the `prv` workspace.

## Overview

`prv-core` defines the foundational data structures used throughout the economic simulation pipeline:

- `State` — 8-dimensional latent economic state vector (`SVector<f64, 8>`) with named accessors
- `Regime` — economic regime classification (`Expansion`, `Saturation`, `Contraction`, `Recovery`, `StructuralShock`)
- `Observation<D>` — generic observed measurement wrapper
- `TimeSeries<T>` — time-indexed series abstraction with interpolation and resampling
- `PressureReleaseValve` — core PRV dynamics with configurable coefficients
- `Control` — type alias for `SVector<f64, 8>` control vector

## Usage

```rust
use prv_core::{State, Regime, Observation, TimeSeries};

let state = State::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0);
assert_eq!(state.capacity(), 1.0);
assert_eq!(state.migration_pressure(), 8.0);

let regime = Regime::from_pressure(&state);
```

## Dependencies

- `nalgebra` — linear algebra types
- `serde` / `serde_json` — serialization
- `chrono` — time handling
- `thiserror` — error definitions

## License

MIT © Metis Avionics
