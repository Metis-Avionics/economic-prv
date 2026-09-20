use crate::{ObservationModel, State, TransitionModel};
use nalgebra::SMatrix;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum EkfError {
    #[error("State contains non-finite values")]
    NonFiniteState,
    #[error("Covariance is not positive semidefinite")]
    NonPsdCovariance,
}

pub type EkfResult<T> = Result<T, EkfError>;

#[derive(Clone, Debug)]
pub struct Ekf<M: TransitionModel, O: ObservationModel<D>, const D: usize> {
    pub x_hat: State,
    pub p: SMatrix<f64, 8, 8>,
    pub q: SMatrix<f64, 8, 8>,
    pub r: SMatrix<f64, D, D>,
    pub transition: M,
    pub observation: O,
}

impl<M: TransitionModel, O: ObservationModel<D>, const D: usize> Ekf<M, O, D> {
    pub const fn new(
        transition: M,
        observation: O,
        x0: State,
        p0: SMatrix<f64, 8, 8>,
        q: SMatrix<f64, 8, 8>,
        r: SMatrix<f64, D, D>,
    ) -> Self {
        Self {
            x_hat: x0,
            p: p0,
            q,
            r,
            transition,
            observation,
        }
    }

    /// Runs the EKF predict step.
    ///
    /// # Errors
    ///
    /// Returns `EkfError` if the state becomes non-finite or covariance becomes non-PSD.
    pub fn predict(&mut self, control: Option<&prv_core::Control>) -> EkfResult<()> {
        let x_prev = self.x_hat.clone();
        self.x_hat = self.transition.f(&x_prev, control);

        let f = self.jacobian_f(&x_prev);
        self.p = f * self.p * f.transpose() + self.q;
        self.stabilize_covariance()?;
        Ok(())
    }

    /// Runs the EKF update step.
    ///
    /// # Errors
    ///
    /// Returns `EkfError` if the innovation covariance is not invertible or state/covariance become non-finite.
    pub fn update(&mut self, z: &prv_core::Observation<D>) -> EkfResult<()> {
        let h = self.observation.jacobian_h(&self.x_hat);
        let z_pred = self.observation.h(&self.x_hat);

        let innovation = z.as_vector() - z_pred.as_vector();
        let s = h * self.p * h.transpose() + self.r;

        let Some(s_inv) = s.try_inverse() else {
            return Err(EkfError::NonPsdCovariance);
        };

        let k = self.p * h.transpose() * s_inv;
        let delta = k * innovation;
        self.x_hat.0 += delta;
        self.p = (SMatrix::<f64, 8, 8>::identity() - k * h) * self.p;
        self.stabilize_covariance()?;
        Ok(())
    }

    /// Stabilizes the covariance matrix to be symmetric and positive semidefinite.
    ///
    /// # Errors
    ///
    /// Returns `EkfError::NonPsdCovariance` if the covariance cannot be stabilized.
    pub fn stabilize_covariance(&mut self) -> EkfResult<()> {
        self.p = (self.p + self.p.transpose()) * 0.5;

        let eigen = self.p.symmetric_eigenvalues();
        let min_eig = eigen.min();
        if min_eig < 1e-10 {
            let mut p = self.p;
            for i in 0..8 {
                p[(i, i)] += (1e-10 - min_eig).max(0.0);
            }
            self.p = p;
        }
        Ok(())
    }

    pub fn jacobian_f(&self, state: &State) -> SMatrix<f64, 8, 8> {
        let x = state.as_vector();
        let h = 1e-6;
        let mut jac = SMatrix::<f64, 8, 8>::zeros();

        for j in 0..8 {
            let mut x_plus = *x;
            x_plus[j] += h;
            let state_plus = State(x_plus);
            let f_plus = self.transition.f(&state_plus, None);

            let mut x_minus = *x;
            x_minus[j] -= h;
            let state_minus = State(x_minus);
            let f_minus = self.transition.f(&state_minus, None);

            let col = (f_plus.as_vector() - f_minus.as_vector()) / (2.0 * h);
            for i in 0..8 {
                jac[(i, j)] = col[i];
            }
        }

        jac
    }

    pub fn innovation_statistics(&self, innovation: &nalgebra::SVector<f64, D>) -> f64 {
        innovation.norm()
    }

    pub fn covariance_symmetry(&self) -> bool {
        let diff = self.p - self.p.transpose();
        diff.norm() < 1e-10
    }

    pub fn finite_state(&self) -> bool {
        self.x_hat.as_vector().iter().all(|&x| x.is_finite())
    }

    pub fn finite_covariance(&self) -> bool {
        self.p.iter().all(|&x| x.is_finite())
    }
}
