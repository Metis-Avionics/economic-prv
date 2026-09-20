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
