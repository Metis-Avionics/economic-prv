use crate::State;
use nalgebra::{Quaternion, SVector, UnitQuaternion};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuaternionState {
    pub state: [f64; 4],
}

impl QuaternionState {
    #[must_use]
    pub const fn new(state: [f64; 4]) -> Self {
        Self { state }
    }

    #[must_use]
    pub fn from_state(state: &State) -> Self {
        let x = state.capacity();
        let y = state.investment();
        let z = state.demand_pressure();
        let w = state.housing_pressure();
        Self::new([x, y, z, w])
    }

    #[must_use]
    pub fn to_state(&self) -> State {
        let x = self.state[0];
        let y = self.state[1];
        let z = self.state[2];
        let w = self.state[3];
        State::new(x, y, 0.0, 0.0, z, w, 0.0, 0.0)
    }

    #[must_use]
    pub fn to_quaternion(&self) -> UnitQuaternion<f64> {
        let [x, y, z, w] = self.state;
        let quat = Quaternion::new(w, x, y, z);
        if quat.vector().norm() < 1e-10 {
            return UnitQuaternion::new_normalize(Quaternion::new(1.0, 0.0, 0.0, 0.0));
        }
        UnitQuaternion::new_normalize(quat)
    }

    #[must_use]
    pub fn transition(
        &self,
        q_prev: &UnitQuaternion<f64>,
        q_curr: &UnitQuaternion<f64>,
    ) -> UnitQuaternion<f64> {
        let q_delta = q_prev.inverse() * q_curr;
        UnitQuaternion::new_normalize(q_delta.into_inner())
    }

    #[must_use]
    pub fn drift_within_threshold(
        &self,
        q_prev: &UnitQuaternion<f64>,
        q_curr: &UnitQuaternion<f64>,
    ) -> bool {
        let q_delta = q_prev.inverse() * q_curr;
        let angle = q_delta.angle();
        angle <= 1e-6
    }

    #[must_use]
    pub fn angle_of_rotation(&self, q_delta: &UnitQuaternion<f64>) -> f64 {
        2.0 * q_delta.angle()
    }

    #[must_use]
    pub fn baseline_compare(
        &self,
        q_results: &prv_monte_carlo::MonteCarloResults,
        linear_results: &prv_monte_carlo::MonteCarloResults,
    ) -> Comparison {
        let q_state = Self::from_state(&q_results.mean);
        let linear_state = Self::from_state(&linear_results.mean);
        let q_vec = SVector::<f64, 4>::from_vec(q_state.state.to_vec());
        let linear_vec = SVector::<f64, 4>::from_vec(linear_state.state.to_vec());
        let diff = (q_vec - linear_vec).norm();
        let comparable = diff < 1.0;
        Comparison {
            quaternion_model_comparable: comparable,
            notes: format!(
                "Quaternion vs linear 4-dim state norm diff: {diff:.6}. {}",
                if comparable {
                    "Results are comparable within threshold"
                } else {
                    "Results diverge beyond threshold"
                }
            ),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Comparison {
    pub quaternion_model_comparable: bool,
    pub notes: String,
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::suboptimal_flops,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::cast_sign_loss
)]
mod tests {
    use super::*;
    use nalgebra::SMatrix;
    use prv_monte_carlo::Simulator;

    #[test]
    fn baseline_compare_returns_comparison() {
        let qs = QuaternionState::from_state(&State::default());
        let simulator = Simulator::new(42);
        let mean = State::default();
        let covariance = SMatrix::<f64, 8, 8>::identity() * 0.1;
        let q_results = simulator.simulate(&mean, &covariance, 10, 3, &[]).unwrap();
        let linear_results = simulator.simulate(&mean, &covariance, 10, 3, &[]).unwrap();
        let comparison = qs.baseline_compare(&q_results, &linear_results);
        assert!(comparison.quaternion_model_comparable);
        assert!(!comparison.notes.is_empty());
    }

    #[test]
    fn drift_within_threshold_for_same_quaternion() {
        let qs = QuaternionState::from_state(&State::default());
        let q = UnitQuaternion::new_normalize(Quaternion::new(1.0, 0.0, 0.0, 0.0));
        assert!(qs.drift_within_threshold(&q, &q));
    }

    #[test]
    fn from_state_maps_correctly() {
        let state = State::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0);
        let qs = QuaternionState::from_state(&state);
        assert_eq!(qs.state[0], 1.0);
        assert_eq!(qs.state[1], 2.0);
        assert_eq!(qs.state[2], 5.0);
        assert_eq!(qs.state[3], 6.0);
    }

    #[test]
    fn to_quaternion_rejects_zero_quaternion() {
        let qs = QuaternionState::new([0.0, 0.0, 0.0, 0.0]);
        let q = qs.to_quaternion();
        assert!(q.quaternion().norm() > 0.99);
    }

    #[test]
    fn to_state_round_trip() {
        let state = State::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0);
        let qs = QuaternionState::from_state(&state);
        let recovered = qs.to_state();
        assert_eq!(recovered.capacity(), 1.0);
        assert_eq!(recovered.investment(), 2.0);
        assert_eq!(recovered.demand_pressure(), 5.0);
        assert_eq!(recovered.housing_pressure(), 6.0);
    }
}
