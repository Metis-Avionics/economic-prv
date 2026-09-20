#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro,
    clippy::use_debug
)]

mod quaternion;

pub use quaternion::{Comparison, QuaternionState};

pub use prv_core::State;
pub use prv_monte_carlo::MonteCarloResults;
