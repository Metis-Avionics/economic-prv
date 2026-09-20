use crate::Observation;
use nalgebra::SMatrix;

pub trait ObservationModel<const D: usize> {
    fn h(&self, state: &crate::State) -> Observation<D>;
    fn jacobian_h(&self, state: &crate::State) -> SMatrix<f64, D, 8>;
    fn r(&self) -> SMatrix<f64, D, D>;
}
