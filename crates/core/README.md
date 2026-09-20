# prv-core

Core domain types, state, regime, observation, and time-series abstractions for the `prv` workspace.

## Overview

`prv-core` defines the foundational data structures used throughout the economic simulation pipeline, including:

- `State` — the base state vector for the PRV model
- `Regime` — economic regime classification
- `Observation` — observed measurement interface
- `TimeSeries` — time-indexed series abstraction
- `PressureReleaseValve` — core PRV dynamics
- `Control` — 8-dimensional control vector (`SVector<f64, 8>`)

## Dependencies

- `nalgebra` — linear algebra types
- `serde` / `serde_json` — serialization
- `chrono` — time handling
- `thiserror` — error definitions

## License

MIT © Metis Avionics
