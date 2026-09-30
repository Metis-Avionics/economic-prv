#![forbid(unsafe_code)]

pub mod backtest;
pub mod metrics;
pub mod tail;

pub use backtest::{Baseline, Evaluator, Model};
pub use metrics::MetricResults;
pub use tail::{MetricError, expected_shortfall, value_at_risk};

pub use prv_core::{State, TimeSeries};
pub use prv_data::DataFrame;
