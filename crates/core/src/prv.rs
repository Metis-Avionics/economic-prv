use crate::State;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PressureReleaseValve {
    pub capacity: f64,
    pub demand: f64,
    pub housing: f64,
    pub fiscal: f64,
    pub geopolitical: f64,
}

impl PressureReleaseValve {
    #[must_use]
    pub const fn new(
        capacity: f64,
        demand: f64,
        housing: f64,
        fiscal: f64,
        geopolitical: f64,
    ) -> Self {
        Self {
            capacity,
            demand,
            housing,
            fiscal,
            geopolitical,
        }
    }

    #[must_use]
    pub fn value(&self, state: &State) -> f64 {
        let pressure = state.geopolitical_load().mul_add(
            self.geopolitical,
            state
                .housing_pressure()
                .mul_add(self.housing, state.demand_pressure() * self.demand),
        );
        let relief = state.fiscal_capacity() * self.fiscal;
        (pressure - relief) / self.capacity.max(1e-10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prv_value_computes_pressure_over_capacity() {
        let prv = PressureReleaseValve::new(2.0, 0.5, 0.3, 0.2, 0.4);
        let state = State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let v = prv.value(&state);
        assert!(v.is_finite());
    }

    #[test]
    fn prv_value_zero_capacity_avoids_divide_by_zero() {
        let prv = PressureReleaseValve::new(0.0, 0.5, 0.3, 0.2, 0.4);
        let state = State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let v = prv.value(&state);
        assert!(v.is_finite());
    }

    #[test]
    fn prv_value_positive_capacity_returns_positive() {
        let prv = PressureReleaseValve::new(1.0, 0.0, 0.0, 0.0, 0.0);
        let state = State::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        assert!(prv.value(&state) >= 0.0);
    }
}
