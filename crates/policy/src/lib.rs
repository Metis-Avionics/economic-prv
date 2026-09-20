#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro,
    clippy::use_debug
)]

pub mod engine;

pub use engine::{PolicyBias, PolicyDistribution, PolicyEngine, PolicyInstrument, PolicyWeights};

pub use prv_core::{Regime, State};
pub use prv_monte_carlo::MonteCarloResults;
