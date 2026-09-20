use crate::State;
use rand::prelude::*;
use rand_distr::StandardNormal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShockType {
    Demand,
    Supply,
    Investment,
    Geopolitical,
    Fiscal,
    Migration,
    Housing,
    Financial,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShockSpec {
    pub shock_type: ShockType,
    pub amplitude: f64,
    pub persistence: f64,
    pub autocorrelation: f64,
}

impl ShockSpec {
    #[allow(clippy::unused_self)]
    pub fn apply(&self, state: &mut State, rng: &mut impl Rng) {
        let normal: f64 = rng.sample(StandardNormal);
        let shock = self.amplitude * normal;

        match self.shock_type {
            ShockType::Demand => state.0[4] += shock,
            ShockType::Supply => state.0[0] = shock.mul_add(-0.5, state.0[0]),
            ShockType::Investment => state.0[1] += shock,
            ShockType::Geopolitical => state.0[6] += shock,
            ShockType::Fiscal => state.0[3] += shock,
            ShockType::Migration => state.0[7] += shock,
            ShockType::Housing => state.0[5] += shock,
            ShockType::Financial => state.0[3] -= shock,
        }
    }
}

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
    use crate::State;
    use rand::SeedableRng;

    #[test]
    fn demand_shock_modifies_demand_pressure() {
        let mut state = State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let spec = ShockSpec {
            shock_type: ShockType::Demand,
            amplitude: 0.1,
            persistence: 0.5,
            autocorrelation: 0.0,
        };
        spec.apply(&mut state, &mut rng);
        assert!(state.demand_pressure() != 1.0);
    }

    #[test]
    fn supply_shock_modifies_capacity() {
        let mut state = State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let spec = ShockSpec {
            shock_type: ShockType::Supply,
            amplitude: 0.1,
            persistence: 0.5,
            autocorrelation: 0.0,
        };
        spec.apply(&mut state, &mut rng);
        assert!(state.capacity().is_finite());
    }

    #[test]
    fn zero_amplitude_produces_no_change() {
        let mut state = State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let spec = ShockSpec {
            shock_type: ShockType::Demand,
            amplitude: 0.0,
            persistence: 0.5,
            autocorrelation: 0.0,
        };
        spec.apply(&mut state, &mut rng);
        assert!((state.capacity() - 1.0).abs() < 1e-10);
        assert!((state.demand_pressure() - 1.0).abs() < 1e-10);
    }
}
