#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]

pub mod backtest;
pub mod metrics;

pub use backtest::{Baseline, Evaluator, Model};
pub use metrics::MetricResults;

pub use prv_core::{State, TimeSeries};
pub use prv_data::DataFrame;
