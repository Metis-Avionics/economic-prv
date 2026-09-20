use crate::State;
use crate::metrics::MetricResults;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub trait Model {
    fn predict(&self, state: &State) -> State;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Baseline {
    Naive,
    MovingAverage,
    LinearStateModel,
    NonQuaternionStateModel,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvaluationResults {
    pub metrics: MetricResults,
    pub baseline_comparison: HashMap<Baseline, MetricResults>,
}

#[derive(Clone, Debug, Default)]
pub struct Evaluator;

impl Evaluator {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    pub fn backtest(
        &self,
        _model: &dyn Model,
        _data: &prv_core::TimeSeries<State>,
        _window: usize,
    ) -> EvaluationResults {
        EvaluationResults {
            metrics: MetricResults {
                rmse: f64::NAN,
                mae: f64::NAN,
                calibration_score: 0.0,
                brier_score: 0.0,
                log_loss: 0.0,
                regime_detection_accuracy: 0.0,
                false_transition_rate: 0.0,
                tail_risk_error: 0.0,
            },
            baseline_comparison: HashMap::new(),
        }
    }
}
