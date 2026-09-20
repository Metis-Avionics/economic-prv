use crate::State;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Regime {
    Expansion,
    Saturation,
    Contraction,
    Recovery,
    StructuralShock,
}

impl Regime {
    #[must_use]
    pub fn from_pressure(state: &State) -> Self {
        let capacity = state.capacity().max(1e-10);
        let pressure_sum = state.demand_pressure()
            + state.housing_pressure()
            + state.geopolitical_load()
            + state.migration_pressure();
        let ratio = pressure_sum / capacity;

        match ratio {
            r if r > 1.5 => Self::StructuralShock,
            r if r > 1.0 => Self::Saturation,
            r if r > 0.7 => Self::Expansion,
            r if r > 0.4 => Self::Recovery,
            _ => Self::Contraction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regime_classification_structural_shock() {
        let state = State::new(1.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 2.0);
        assert_eq!(Regime::from_pressure(&state), Regime::StructuralShock);
    }

    #[test]
    fn regime_classification_saturation() {
        let state = State::new(1.0, 1.0, 1.0, 1.0, 0.5, 0.5, 0.2, 0.0);
        assert_eq!(Regime::from_pressure(&state), Regime::Saturation);
    }

    #[test]
    fn regime_classification_contraction() {
        let state = State::new(1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(Regime::from_pressure(&state), Regime::Contraction);
    }
}
