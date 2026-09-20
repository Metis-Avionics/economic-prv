# prv-geometry

Quaternion state geometry and comparison utilities for the `prv` workspace.

## Overview

`prv-geometry` provides geometric primitives for state comparison and orientation handling:

- `QuaternionState` — quaternion-based state representation (`[x, y, z, w]`)
- `Comparison` — state comparison result with notes

The crate extends `prv-core` state types with geometric operations and leverages `prv-monte-carlo` for probabilistic geometry sampling.

## State Mapping

The quaternion model maps the 8-dimensional economic state to a 4-dimensional quaternion:

| Quaternion Component | Economic State Field |
|---------------------|----------------------|
| `x` | `capacity` |
| `y` | `investment` |
| `z` | `demand_pressure` |
| `w` | `housing_pressure` |

## Usage

```rust
use prv_geometry::QuaternionState;
use prv_core::State;

let state = State::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0);
let q_state = QuaternionState::from_state(&state);
let quat = q_state.to_quaternion();
let angle = q_state.angle_of_rotation(&quat);

let comparison = q_state.baseline_compare(&q_results, &linear_results);
println!("Comparable: {}", comparison.quaternion_model_comparable);
```

## Validation

- Unit norm enforced after every multiplication
- Zero quaternion rejected (falls back to identity `[1, 0, 0, 0]`)
- Drift threshold: `1e-6`
- Baseline comparison required against non-quaternion linear state model

## Dependencies

- `prv-core` — base state type
- `prv-monte-carlo` — Monte Carlo results for sampling
- `nalgebra` — linear algebra
- `serde` — serialization

## License

MIT © Metis Avionics
