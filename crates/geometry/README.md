# prv-geometry

Quaternion state geometry and comparison utilities for the `prv` workspace.

## Overview

`prv-geometry` provides geometric primitives for state comparison and orientation handling:

- `QuaternionState` — quaternion-based state representation
- `Comparison` — state comparison utilities

The crate extends `prv-core` state types with geometric operations and leverages `prv-monte-carlo` for probabilistic geometry sampling.

## Dependencies

- `prv-core` — base state type
- `prv-monte-carlo` — Monte Carlo results for sampling
- `nalgebra` — linear algebra
- `serde` / `thiserror` / `tracing` — serialization, errors, logging

## License

MIT © Metis Avionics
