use nalgebra::DVector;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Observation(pub DVector<f64>);

impl Serialize for Observation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        debug_assert!(
            self.0.iter().all(|&x| x.is_finite()),
            "observation values must be finite"
        );
        self.0.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Observation {
    fn deserialize<Deserializer>(deserializer: Deserializer) -> Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'de>,
    {
        let vec = Vec::<f64>::deserialize(deserializer)?;
        Ok(Self(DVector::from_vec(vec)))
    }
}

impl Observation {
    #[must_use]
    pub fn new(values: Vec<f64>) -> Self {
        Self(DVector::from_vec(values))
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn as_vector(&self) -> &DVector<f64> {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_new_sets_dimension() {
        let obs = Observation::new(vec![1.0, 2.0, 3.0]);
        assert_eq!(obs.as_vector().len(), 3);
    }

    #[test]
    fn observation_as_vector_returns_reference() {
        let obs = Observation::new(vec![0.0; 10]);
        assert_eq!(obs.as_vector().len(), 10);
    }

    #[test]
    fn observation_serialize_roundtrip() {
        let obs = Observation::new(vec![1.0, 2.0, 3.0]);
        let json = serde_json::to_string(&obs).unwrap();
        let back: Observation = serde_json::from_str(&json).unwrap();
        assert_eq!(obs.as_vector().as_slice(), back.as_vector().as_slice());
    }
}
