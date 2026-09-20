#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]

mod quaternion;

pub use quaternion::{Comparison, QuaternionState};

pub use prv_core::State;
pub use prv_monte_carlo::MonteCarloResults;
