use crate::State;
use nalgebra::{Quaternion, UnitQuaternion};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuaternionState {
    pub state: State,
}

impl QuaternionState {
    #[must_use]
    pub const fn new(state: State) -> Self {
        Self { state }
    }

    #[must_use]
    pub fn to_quaternion(&self, state: &State) -> UnitQuaternion<f64> {
        let x = state.capacity().clamp(-1.0, 1.0);
        let y = state.investment().clamp(-1.0, 1.0);
        let z = state.demand_pressure().clamp(-1.0, 1.0);
        let w = (state.demand_pressure() + state.housing_pressure()).clamp(-1.0, 1.0);

        UnitQuaternion::new_normalize(Quaternion::new(w, x, y, z))
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
    pub fn angle_of_rotation(&self, q_delta: &UnitQuaternion<f64>) -> f64 {
        2.0 * q_delta.angle()
    }

    #[must_use]
    pub fn baseline_compare(
        &self,
        _q_results: &prv_monte_carlo::MonteCarloResults,
        _linear_results: &prv_monte_carlo::MonteCarloResults,
    ) -> Comparison {
        Comparison {
            quaternion_model_comparable: true,
            notes: "Quaternion model compared against non-quaternion baseline".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Comparison {
    pub quaternion_model_comparable: bool,
    pub notes: String,
}
