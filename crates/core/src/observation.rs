use crate::SVector;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Observation<const D: usize>(pub SVector<f64, D>);

impl<const D: usize> Serialize for Observation<D> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.as_slice().serialize(serializer)
    }
}

impl<'de, const D: usize> Deserialize<'de> for Observation<D> {
    fn deserialize<Deserializer>(deserializer: Deserializer) -> Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'de>,
    {
        let vec = Vec::<f64>::deserialize(deserializer)?;
        if vec.len() != D {
            return Err(serde::de::Error::custom(format!(
                "Observation must have {D} elements"
            )));
        }
        Ok(Self(SVector::from_vec(vec)))
    }
}

impl<const D: usize> Observation<D> {
    #[must_use]
    pub fn new(values: [f64; D]) -> Self {
        Self(SVector::from_vec(values.to_vec()))
    }

    #[must_use]
    pub const fn as_vector(&self) -> &SVector<f64, D> {
        &self.0
    }
}
