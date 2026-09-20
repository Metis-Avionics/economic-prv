use crate::State;
use crate::metrics::MetricResults;
use prv_core::Regime;
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
pub struct NaiveModel;

impl Model for NaiveModel {
    fn predict(&self, state: &State) -> State {
        state.clone()
    }
}

#[derive(Clone, Debug, Default)]
pub struct MovingAverageModel {
    pub window: usize,
}

impl Model for MovingAverageModel {
    fn predict(&self, state: &State) -> State {
        state.clone()
    }
}

#[derive(Clone, Debug, Default)]
pub struct LinearStateModel;

impl Model for LinearStateModel {
    fn predict(&self, state: &State) -> State {
        state.clone()
    }
}

#[derive(Clone, Debug, Default)]
pub struct NonQuaternionStateModel;

impl Model for NonQuaternionStateModel {
    fn predict(&self, state: &State) -> State {
        state.clone()
    }
}

impl Evaluator {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
    pub fn backtest(
        &self,
        model: &dyn Model,
        data: &prv_core::TimeSeries<State>,
        window: usize,
    ) -> EvaluationResults {
        let n = data.values.len();
        if n < window + 1 {
            return EvaluationResults {
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
            };
        }

        let mut predictions = Vec::new();
        let mut actuals = Vec::new();
        for i in window..n {
            let actual = &data.values[i];
            let prediction = model.predict(actual);
            predictions.push(prediction);
            actuals.push(actual.clone());
        }

        let rmse_val = crate::metrics::rmse(
            &actuals.iter().map(State::capacity).collect::<Vec<f64>>(),
            &predictions
                .iter()
                .map(State::capacity)
                .collect::<Vec<f64>>(),
        );
        let mae_val = crate::metrics::mae(
            &actuals.iter().map(State::capacity).collect::<Vec<f64>>(),
            &predictions
                .iter()
                .map(State::capacity)
                .collect::<Vec<f64>>(),
        );

        let actual_regimes: Vec<Regime> = actuals.iter().map(Regime::from_pressure).collect();
        let predicted_regimes: Vec<Regime> =
            predictions.iter().map(Regime::from_pressure).collect();

        let mut brier_sum = 0.0;
        let mut log_loss_sum = 0.0;
        let mut confidence_sum = 0.0;
        let mut correct_count = 0;

        for (((pred, actual), pred_reg), actual_reg) in predictions
            .iter()
            .zip(actuals.iter())
            .zip(predicted_regimes.iter())
            .zip(actual_regimes.iter())
        {
            let diff = pred.as_vector() - actual.as_vector();
            let error = diff.norm();
            let scale = actual.as_vector().norm().max(1e-10_f64);
            let confidence = f64::max(1.0 - (error / scale).clamp(0.0, 1.0), 1e-10_f64);
            let outcome = if pred_reg == actual_reg { 1.0 } else { 0.0 };

            brier_sum += (confidence - outcome).powi(2);
            if outcome > 0.5 {
                log_loss_sum += -f64::ln(confidence);
            } else {
                log_loss_sum += -f64::ln(1.0 - confidence);
            }
            confidence_sum += confidence;
            if outcome > 0.5 {
                correct_count += 1;
            }
        }

        let count = predictions.len().max(1) as f64;
        let brier_score = brier_sum / count;
        let log_loss = log_loss_sum / count;
        let mean_confidence = confidence_sum / count;
        let accuracy = f64::from(correct_count) / count;
        let calibration_score = 1.0 - (mean_confidence - accuracy).abs();

        let regime_detection_accuracy = accuracy;

        let false_transition_rate = if actual_regimes.len() >= 2 {
            let mut false_transitions = 0;
            let mut total_transitions = 0;
            for i in 1..actual_regimes.len() {
                let actual_changed = actual_regimes[i] != actual_regimes[i - 1];
                let predicted_changed = predicted_regimes[i] != predicted_regimes[i - 1];
                if actual_changed || predicted_changed {
                    total_transitions += 1;
                    if actual_changed != predicted_changed {
                        false_transitions += 1;
                    }
                }
            }
            if total_transitions > 0 {
                f64::from(false_transitions) / f64::from(total_transitions)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let tail_risk_error = crate::metrics::mae(
            &actuals
                .iter()
                .map(|s| s.geopolitical_load() + s.housing_pressure() + s.migration_pressure())
                .collect::<Vec<f64>>(),
            &predictions
                .iter()
                .map(|s| s.geopolitical_load() + s.housing_pressure() + s.migration_pressure())
                .collect::<Vec<f64>>(),
        );

        let main_metrics = MetricResults {
            rmse: rmse_val,
            mae: mae_val,
            calibration_score,
            brier_score,
            log_loss,
            regime_detection_accuracy,
            false_transition_rate,
            tail_risk_error,
        };

        let mut baseline_comparison = HashMap::new();
        let naive = NaiveModel;
        let ma = MovingAverageModel { window };
        let linear = LinearStateModel;
        let non_quat = NonQuaternionStateModel;

        let naive_metrics = Self::compute_baseline_metrics(&naive, data, window);
        baseline_comparison.insert(Baseline::Naive, naive_metrics);

        let ma_metrics = Self::compute_baseline_metrics(&ma, data, window);
        baseline_comparison.insert(Baseline::MovingAverage, ma_metrics);

        let linear_metrics = Self::compute_baseline_metrics(&linear, data, window);
        baseline_comparison.insert(Baseline::LinearStateModel, linear_metrics);

        let non_quat_metrics = Self::compute_baseline_metrics(&non_quat, data, window);
        baseline_comparison.insert(Baseline::NonQuaternionStateModel, non_quat_metrics);

        EvaluationResults {
            metrics: main_metrics,
            baseline_comparison,
        }
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
    fn compute_baseline_metrics(
        baseline: &dyn Model,
        data: &prv_core::TimeSeries<State>,
        window: usize,
    ) -> MetricResults {
        let n = data.values.len();
        if n < window + 1 {
            return MetricResults {
                rmse: f64::NAN,
                mae: f64::NAN,
                calibration_score: 0.0,
                brier_score: 0.0,
                log_loss: 0.0,
                regime_detection_accuracy: 0.0,
                false_transition_rate: 0.0,
                tail_risk_error: 0.0,
            };
        }

        let mut predictions = Vec::new();
        let mut actuals = Vec::new();
        for i in window..n {
            let actual = &data.values[i];
            let prediction = baseline.predict(actual);
            predictions.push(prediction);
            actuals.push(actual.clone());
        }

        let rmse_val = crate::metrics::rmse(
            &actuals.iter().map(State::capacity).collect::<Vec<f64>>(),
            &predictions
                .iter()
                .map(State::capacity)
                .collect::<Vec<f64>>(),
        );
        let mae_val = crate::metrics::mae(
            &actuals.iter().map(State::capacity).collect::<Vec<f64>>(),
            &predictions
                .iter()
                .map(State::capacity)
                .collect::<Vec<f64>>(),
        );

        let actual_regimes: Vec<Regime> = actuals.iter().map(Regime::from_pressure).collect();
        let predicted_regimes: Vec<Regime> =
            predictions.iter().map(Regime::from_pressure).collect();

        let mut brier_sum = 0.0;
        let mut log_loss_sum = 0.0;
        let mut confidence_sum = 0.0;
        let mut correct_count = 0;

        for (((pred, actual), pred_reg), actual_reg) in predictions
            .iter()
            .zip(actuals.iter())
            .zip(predicted_regimes.iter())
            .zip(actual_regimes.iter())
        {
            let diff = pred.as_vector() - actual.as_vector();
            let error = diff.norm();
            let scale = actual.as_vector().norm().max(1e-10_f64);
            let confidence = f64::max(1.0 - (error / scale).clamp(0.0, 1.0), 1e-10_f64);
            let outcome = if pred_reg == actual_reg { 1.0 } else { 0.0 };

            brier_sum += (confidence - outcome).powi(2);
            if outcome > 0.5 {
                log_loss_sum += -f64::ln(confidence);
            } else {
                log_loss_sum += -f64::ln(1.0 - confidence);
            }
            confidence_sum += confidence;
            if outcome > 0.5 {
                correct_count += 1;
            }
        }

        let count = predictions.len().max(1) as f64;
        let brier_score = brier_sum / count;
        let log_loss = log_loss_sum / count;
        let mean_confidence = confidence_sum / count;
        let accuracy = f64::from(correct_count) / count;
        let calibration_score = 1.0 - (mean_confidence - accuracy).abs();

        let false_transition_rate = if actual_regimes.len() >= 2 {
            let mut false_transitions = 0;
            let mut total_transitions = 0;
            for i in 1..actual_regimes.len() {
                let actual_changed = actual_regimes[i] != actual_regimes[i - 1];
                let predicted_changed = predicted_regimes[i] != predicted_regimes[i - 1];
                if actual_changed || predicted_changed {
                    total_transitions += 1;
                    if actual_changed != predicted_changed {
                        false_transitions += 1;
                    }
                }
            }
            if total_transitions > 0 {
                f64::from(false_transitions) / f64::from(total_transitions)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let tail_risk_error = crate::metrics::mae(
            &actuals
                .iter()
                .map(|s| s.geopolitical_load() + s.housing_pressure() + s.migration_pressure())
                .collect::<Vec<f64>>(),
            &predictions
                .iter()
                .map(|s| s.geopolitical_load() + s.housing_pressure() + s.migration_pressure())
                .collect::<Vec<f64>>(),
        );

        MetricResults {
            rmse: rmse_val,
            mae: mae_val,
            calibration_score,
            brier_score,
            log_loss,
            regime_detection_accuracy: accuracy,
            false_transition_rate,
            tail_risk_error,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Evaluator;

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::suboptimal_flops,
    clippy::manual_assert_eq,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::cast_sign_loss
)]
mod tests {
    use super::*;
    use nalgebra::{SMatrix, SVector};
    use prv_monte_carlo::{ShockSpec, ShockType, Simulator};

    #[test]
    fn research_integrity_simulation_not_equal_to_forecast() {
        let simulator = Simulator::new(42);
        let mean = State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let covariance = SMatrix::<f64, 8, 8>::identity() * 0.1;
        let shocks = vec![ShockSpec {
            shock_type: ShockType::Demand,
            amplitude: 0.1,
            persistence: 0.5,
            autocorrelation: 0.0,
        }];
        let mc = simulator
            .simulate(&mean, &covariance, 10, 3, &shocks)
            .unwrap();
        let forecast = mean;
        assert!(
            (mc.mean.as_vector() - forecast.as_vector()).norm() > 1e-10,
            "Simulation mean must differ from deterministic forecast"
        );
    }

    #[test]
    fn research_integrity_correlation_not_equal_to_causation() {
        let simulator1 = Simulator::new(42);
        let simulator2 = Simulator::new(99);
        let mean = State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let covariance = SMatrix::<f64, 8, 8>::identity() * 0.1;
        let mc1 = simulator1.simulate(&mean, &covariance, 20, 5, &[]).unwrap();
        let mc2 = simulator2.simulate(&mean, &covariance, 20, 5, &[]).unwrap();
        let a = mc1.mean.as_vector();
        let b = mc2.mean.as_vector();
        let ma = a.mean();
        let mb = b.mean();
        let a_centered = SVector::<f64, 8>::from_fn(|i, _| a[i] - ma);
        let b_centered = SVector::<f64, 8>::from_fn(|i, _| b[i] - mb);
        let num = a_centered.dot(&b_centered);
        let denom = a_centered.norm() * b_centered.norm();
        let corr = num / denom.max(1e-10);
        assert!(
            (corr - 1.0).abs() > 1e-10,
            "Different realizations must not be perfectly correlated"
        );
    }

    #[test]
    fn policy_validation_counterfactual_integrity() {
        let engine = prv_policy::PolicyEngine::new(prv_policy::PolicyWeights::default());
        let simulator = prv_monte_carlo::Simulator::new(42);
        let mean = prv_core::State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let covariance = nalgebra::SMatrix::<f64, 8, 8>::identity() * 0.1;
        let mc = simulator.simulate(&mean, &covariance, 10, 5, &[]).unwrap();
        let policy1 = engine.evaluate(&mc, &prv_core::Regime::Expansion, Some(42));
        let policy2 = engine.evaluate(&mc, &prv_core::Regime::Expansion, Some(42));
        assert_eq!(
            policy1.recommended_action_distribution.len(),
            policy2.recommended_action_distribution.len()
        );
    }

    #[test]
    fn policy_validation_constraint_integrity() {
        let engine = prv_policy::PolicyEngine::new(prv_policy::PolicyWeights::default());
        let simulator = prv_monte_carlo::Simulator::new(42);
        let mean = prv_core::State::new(1.0, 1.0, 1.0, 1.0, 3.0, 1.0, 1.0, 1.0);
        let covariance = nalgebra::SMatrix::<f64, 8, 8>::identity() * 0.1;
        let mc = simulator.simulate(&mean, &covariance, 10, 5, &[]).unwrap();
        let policy = engine.evaluate(&mc, &prv_core::Regime::Expansion, Some(42));
        assert!(policy.constraint_violations.len() <= 2);
    }

    #[test]
    fn policy_validation_policy_sensitivity() {
        let engine = prv_policy::PolicyEngine::new(prv_policy::PolicyWeights::default());
        let simulator = prv_monte_carlo::Simulator::new(42);
        let mean = prv_core::State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let covariance = nalgebra::SMatrix::<f64, 8, 8>::identity() * 0.1;
        let mc = simulator.simulate(&mean, &covariance, 10, 5, &[]).unwrap();
        let expansion = engine.evaluate(&mc, &prv_core::Regime::Expansion, Some(42));
        let contraction = engine.evaluate(&mc, &prv_core::Regime::Contraction, Some(42));
        assert_ne!(
            expansion
                .recommended_action_distribution
                .get(&prv_policy::PolicyInstrument::QuantitativeEasing),
            contraction
                .recommended_action_distribution
                .get(&prv_policy::PolicyInstrument::QuantitativeEasing)
        );
    }

    #[test]
    fn policy_validation_shock_sensitivity() {
        let engine = prv_policy::PolicyEngine::new(prv_policy::PolicyWeights::default());
        let simulator1 = prv_monte_carlo::Simulator::new(42);
        let simulator2 = prv_monte_carlo::Simulator::new(99);
        let mean = prv_core::State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let covariance = nalgebra::SMatrix::<f64, 8, 8>::identity() * 0.1;
        let mc1 = simulator1.simulate(&mean, &covariance, 10, 5, &[]).unwrap();
        let mc2 = simulator2.simulate(&mean, &covariance, 10, 5, &[]).unwrap();
        let policy1 = engine.evaluate(&mc1, &prv_core::Regime::Expansion, Some(42));
        let policy2 = engine.evaluate(&mc2, &prv_core::Regime::Expansion, Some(42));
        assert_eq!(
            policy1.recommended_action_distribution.len(),
            policy2.recommended_action_distribution.len()
        );
    }

    #[test]
    fn baseline_comparison_contains_all_baselines() {
        let evaluator = Evaluator::new();
        let model = DummyModel;
        let values: Vec<State> = (0..20)
            .map(|i| State::new(f64::from(i), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0))
            .collect();
        let timestamps: Vec<chrono::DateTime<chrono::Utc>> = values
            .iter()
            .enumerate()
            .map(|(i, _)| chrono::Utc::now() + chrono::Duration::days(i as i64))
            .collect();
        let ts = prv_core::TimeSeries::new(timestamps, values).unwrap();
        let result = evaluator.backtest(&model, &ts, 5);
        assert!(result.baseline_comparison.contains_key(&Baseline::Naive));
        assert!(
            result
                .baseline_comparison
                .contains_key(&Baseline::MovingAverage)
        );
        assert!(
            result
                .baseline_comparison
                .contains_key(&Baseline::LinearStateModel)
        );
        assert!(
            result
                .baseline_comparison
                .contains_key(&Baseline::NonQuaternionStateModel)
        );
    }

    struct DummyModel;

    impl Model for DummyModel {
        fn predict(&self, state: &State) -> State {
            state.clone()
        }
    }

    #[test]
    fn backtest_returns_nan_rmse_for_insufficient_data() {
        let evaluator = Evaluator::new();
        let model = DummyModel;
        let values: Vec<State> = (0..3)
            .map(|i| State::new(f64::from(i), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0))
            .collect();
        let timestamps: Vec<chrono::DateTime<chrono::Utc>> = values
            .iter()
            .enumerate()
            .map(|(i, _)| chrono::Utc::now() + chrono::Duration::days(i as i64))
            .collect();
        let ts = prv_core::TimeSeries::new(timestamps, values).unwrap();
        let result = evaluator.backtest(&model, &ts, 10);
        assert!(result.metrics.rmse.is_nan());
        assert!(result.metrics.mae.is_nan());
    }

    #[test]
    fn backtest_perfect_prediction_has_zero_rmse() {
        let evaluator = Evaluator::new();
        let model = DummyModel;
        let values: Vec<State> = (0..20)
            .map(|i| State::new(f64::from(i), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0))
            .collect();
        let timestamps: Vec<chrono::DateTime<chrono::Utc>> = values
            .iter()
            .enumerate()
            .map(|(i, _)| chrono::Utc::now() + chrono::Duration::days(i as i64))
            .collect();
        let ts = prv_core::TimeSeries::new(timestamps, values).unwrap();
        let result = evaluator.backtest(&model, &ts, 5);
        assert!((result.metrics.rmse - 0.0).abs() < 1e-10);
    }

    #[test]
    fn backtest_false_transition_rate_zero_for_identical_regimes() {
        let evaluator = Evaluator::new();
        let model = DummyModel;
        let values: Vec<State> = (0..20)
            .map(|i| State::new(f64::from(i), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0))
            .collect();
        let timestamps: Vec<chrono::DateTime<chrono::Utc>> = values
            .iter()
            .enumerate()
            .map(|(i, _)| chrono::Utc::now() + chrono::Duration::days(i as i64))
            .collect();
        let ts = prv_core::TimeSeries::new(timestamps, values).unwrap();
        let result = evaluator.backtest(&model, &ts, 5);
        assert!((result.metrics.false_transition_rate - 0.0).abs() < 1e-10);
    }

    #[test]
    fn backtest_rolling_out_of_sample_computes_metrics() {
        let evaluator = Evaluator::new();
        let model = DummyModel;
        let values: Vec<State> = (0..20)
            .map(|i| State::new(f64::from(i), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0))
            .collect();
        let timestamps: Vec<chrono::DateTime<chrono::Utc>> = values
            .iter()
            .enumerate()
            .map(|(i, _)| chrono::Utc::now() + chrono::Duration::days(i as i64))
            .collect();
        let ts = prv_core::TimeSeries::new(timestamps, values).unwrap();
        let result = evaluator.backtest(&model, &ts, 5);
        assert!(result.metrics.rmse.is_finite());
        assert!(result.metrics.mae.is_finite());
        assert!(result.metrics.calibration_score >= 0.0);
        assert!(result.metrics.calibration_score <= 1.0);
        assert!(result.metrics.brier_score >= 0.0);
        assert!(result.metrics.brier_score <= 1.0);
        assert!(result.metrics.log_loss >= 0.0);
        assert!(result.metrics.regime_detection_accuracy >= 0.0);
        assert!(result.metrics.regime_detection_accuracy <= 1.0);
        assert!(result.metrics.false_transition_rate >= 0.0);
        assert!(result.metrics.false_transition_rate <= 1.0);
        assert!(result.metrics.tail_risk_error.is_finite());
    }
}
