use crate::SVector;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct State(pub SVector<f64, 8>);

impl Serialize for State {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for State {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vec = Vec::<f64>::deserialize(deserializer)?;
        if vec.len() != 8 {
            return Err(serde::de::Error::custom("State must have 8 elements"));
        }
        Ok(Self(SVector::from_vec(vec)))
    }
}

impl State {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        capacity: f64,
        investment: f64,
        labour_absorption: f64,
        fiscal_capacity: f64,
        demand_pressure: f64,
        housing_pressure: f64,
        geopolitical_load: f64,
        migration_pressure: f64,
    ) -> Self {
        Self(SVector::from_vec(vec![
            capacity,
            investment,
            labour_absorption,
            fiscal_capacity,
            demand_pressure,
            housing_pressure,
            geopolitical_load,
            migration_pressure,
        ]))
    }

    #[must_use]
    pub fn capacity(&self) -> f64 {
        self.0[0]
    }

    #[must_use]
    pub fn investment(&self) -> f64 {
        self.0[1]
    }

    #[must_use]
    pub fn labour_absorption(&self) -> f64 {
        self.0[2]
    }

    #[must_use]
    pub fn fiscal_capacity(&self) -> f64 {
        self.0[3]
    }

    #[must_use]
    pub fn demand_pressure(&self) -> f64 {
        self.0[4]
    }

    #[must_use]
    pub fn housing_pressure(&self) -> f64 {
        self.0[5]
    }

    #[must_use]
    pub fn geopolitical_load(&self) -> f64 {
        self.0[6]
    }

    #[must_use]
    pub fn migration_pressure(&self) -> f64 {
        self.0[7]
    }

    #[must_use]
    pub const fn as_vector(&self) -> &SVector<f64, 8> {
        &self.0
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    }
}
