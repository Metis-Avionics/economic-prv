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

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn log_loss(probabilities: &[f64], outcomes: &[bool]) -> f64 {
    if probabilities.len() != outcomes.len() || probabilities.is_empty() {
        return f64::NAN;
    }
    let mut sum = 0.0;
    for (&p, &o) in probabilities.iter().zip(outcomes.iter()) {
        let p_clamped = p.clamp(1e-10, 1.0 - 1e-10);
        sum += if o {
            -p_clamped.ln()
        } else {
            -(1.0 - p_clamped).ln()
        };
    }
    sum / probabilities.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rmse_identical_vectors_is_zero() {
        let actual = vec![1.0, 2.0, 3.0];
        let predicted = vec![1.0, 2.0, 3.0];
        assert!((rmse(&actual, &predicted) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn rmse_mismatched_lengths_is_nan() {
        assert!(rmse(&[1.0], &[1.0, 2.0]).is_nan());
    }

    #[test]
    fn rmse_empty_is_nan() {
        assert!(rmse(&[], &[]).is_nan());
    }

    #[test]
    fn mae_identical_vectors_is_zero() {
        let actual = vec![1.0, 2.0, 3.0];
        let predicted = vec![1.0, 2.0, 3.0];
        assert!((mae(&actual, &predicted) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn brier_score_perfect_forecast_is_zero() {
        let probs = vec![1.0, 1.0, 1.0];
        let outcomes = vec![true, true, true];
        assert!((brier_score(&probs, &outcomes) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn log_loss_perfect_forecast_is_near_zero() {
        let probs = vec![1.0 - 1e-12; 4];
        let outcomes = vec![true, true, true, true];
        assert!(log_loss(&probs, &outcomes) < 1e-9);
    }

    #[test]
    fn log_loss_mismatched_lengths_is_nan() {
        assert!(log_loss(&[0.5], &[true, false]).is_nan());
    }
}
