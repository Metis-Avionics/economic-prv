use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricResults {
    pub rmse: f64,
    pub mae: f64,
    pub calibration_score: f64,
    pub brier_score: f64,
    pub log_loss: f64,
    pub regime_detection_accuracy: f64,
    pub false_transition_rate: f64,
    pub tail_risk_error: f64,
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn rmse(actual: &[f64], predicted: &[f64]) -> f64 {
    if actual.len() != predicted.len() || actual.is_empty() {
        return f64::NAN;
    }
    let sum_sq: f64 = actual
        .iter()
        .zip(predicted.iter())
        .map(|(a, p)| (a - p).powi(2))
        .sum();
    (sum_sq / actual.len() as f64).sqrt()
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn mae(actual: &[f64], predicted: &[f64]) -> f64 {
    if actual.len() != predicted.len() || actual.is_empty() {
        return f64::NAN;
    }
    let sum_abs: f64 = actual
        .iter()
        .zip(predicted.iter())
        .map(|(a, p)| (a - p).abs())
        .sum();
    sum_abs / actual.len() as f64
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn brier_score(probabilities: &[f64], outcomes: &[bool]) -> f64 {
    if probabilities.len() != outcomes.len() || probabilities.is_empty() {
        return f64::NAN;
    }
    let sum: f64 = probabilities
        .iter()
        .zip(outcomes.iter())
        .map(|(p, &o)| (p - if o { 1.0 } else { 0.0 }).powi(2))
        .sum();
    sum / probabilities.len() as f64
}
