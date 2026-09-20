use crate::{Observation, State};
use nalgebra::DMatrix;

pub trait ObservationModel {
    fn h(&self, state: &State) -> Observation;
    fn jacobian_h(&self, state: &State) -> DMatrix<f64>;
    fn r(&self) -> DMatrix<f64>;
}

#[derive(Clone, Debug, Default)]
pub struct DefaultObservation {
    dim: usize,
}

impl DefaultObservation {
    #[must_use]
    pub const fn new() -> Self {
        Self { dim: 0 }
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_dim(mut self, dim: usize) -> Self {
        self.dim = dim;
        self
    }
}

impl ObservationModel for DefaultObservation {
    fn h(&self, _state: &State) -> Observation {
        Observation::new(vec![0.0; self.dim])
    }

    fn jacobian_h(&self, _state: &State) -> DMatrix<f64> {
        DMatrix::<f64>::zeros(self.dim, 8)
    }

    fn r(&self) -> DMatrix<f64> {
        DMatrix::<f64>::identity(self.dim, self.dim)
    }
}
