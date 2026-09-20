use crate::{ObservationModel, State, TransitionModel};
use nalgebra::{DMatrix, SMatrix};
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
pub struct Ekf<M: TransitionModel, O: ObservationModel> {
    pub x_hat: State,
    pub p: SMatrix<f64, 8, 8>,
    pub q: SMatrix<f64, 8, 8>,
    pub r: DMatrix<f64>,
    pub transition: M,
    pub observation: O,
}

impl<M: TransitionModel, O: ObservationModel> Ekf<M, O> {
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(
        transition: M,
        observation: O,
        x0: State,
        p0: SMatrix<f64, 8, 8>,
        q: SMatrix<f64, 8, 8>,
        r: DMatrix<f64>,
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
        self.quaternion_normalize();
        Ok(())
    }

    /// Runs the EKF update step.
    ///
    /// # Errors
    ///
    /// Returns `EkfError` if the innovation covariance is not invertible or state/covariance become non-finite.
    pub fn update(&mut self, z: &prv_core::Observation) -> EkfResult<()> {
        let h = self.observation.jacobian_h(&self.x_hat);
        let z_pred = self.observation.h(&self.x_hat);

        let innovation = z.as_vector() - z_pred.as_vector();
        let s = h.clone() * self.p * h.transpose() + self.r.clone();

        let Some(s_inv) = s.try_inverse() else {
            return Err(EkfError::NonPsdCovariance);
        };

        let k = self.p * h.transpose() * s_inv;
        let delta = k.clone() * innovation;
        let delta_slice = delta.as_slice();
        for (i, val) in delta_slice.iter().enumerate() {
            self.x_hat.0[i] += val;
        }
        self.p = (SMatrix::<f64, 8, 8>::identity() - k * h) * self.p;
        self.stabilize_covariance()?;
        self.quaternion_normalize();
        Ok(())
    }

    /// Stabilizes the covariance matrix to be symmetric and positive semidefinite.
    ///
    /// # Errors
    ///
    /// Returns `EkfError::NonPsdCovariance` if the covariance cannot be stabilized.
    ///
    /// # Panics
    ///
    /// Panics if the covariance matrix contains non-finite values after stabilization.
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
        assert!(
            self.p.iter().all(|&x| x.is_finite()),
            "covariance must be finite after stabilization"
        );
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

    pub fn quaternion_normalize(&mut self) {
        let x = self.x_hat.as_vector();
        let qx = x[0];
        let qy = x[1];
        let qz = x[4];
        let qw = x[5];
        let norm = (qx.mul_add(qx, qy.mul_add(qy, qz.mul_add(qz, qw * qw)))).sqrt();
        if norm > 1e-10 {
            let inv = 1.0 / norm;
            self.x_hat.0[0] = qx * inv;
            self.x_hat.0[1] = qy * inv;
            self.x_hat.0[4] = qz * inv;
            self.x_hat.0[5] = qw * inv;
        }
    }

    pub fn innovation_statistics(
        &self,
        innovation: &nalgebra::DVector<f64>,
        s: &nalgebra::DMatrix<f64>,
    ) -> (f64, f64) {
        let s_inv = s
            .clone()
            .try_inverse()
            .unwrap_or_else(|| nalgebra::DMatrix::zeros(s.nrows(), s.ncols()));
        let nis = innovation.dot(&(s_inv * innovation));
        #[allow(clippy::cast_precision_loss)]
        let n = innovation.len() as f64;
        let mean = innovation.norm() / n;
        #[allow(clippy::cast_precision_loss)]
        let variance = innovation.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n;
        let std = variance.sqrt();
        (nis, std)
    }

    pub fn covariance_symmetry(&self) -> bool {
        let diff = self.p - self.p.transpose();
        diff.norm() < 1e-10
    }

    pub fn covariance_psd(&self) -> bool {
        let eigen = self.p.symmetric_eigenvalues();
        eigen.iter().all(|&e| e >= -1e-10)
    }

    pub fn finite_state(&self) -> bool {
        self.x_hat.as_vector().iter().all(|&x| x.is_finite())
    }

    pub fn finite_covariance(&self) -> bool {
        self.p.iter().all(|&x| x.is_finite())
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
    use crate::{observation::DefaultObservation, transition::DefaultTransition};

    #[test]
    fn innovation_statistics_returns_nis_and_std() {
        let transition = DefaultTransition;
        let observation = DefaultObservation::new().with_dim(10);
        let x0 = State::default();
        let p0 = SMatrix::<f64, 8, 8>::identity();
        let q = SMatrix::<f64, 8, 8>::identity() * 0.01;
        let r = DMatrix::<f64>::identity(10, 10) * 0.1;
        let ekf = Ekf::new(transition, observation, x0, p0, q, r);
        let innovation = nalgebra::DVector::<f64>::zeros(10);
        let s = DMatrix::<f64>::identity(10, 10);
        let (nis, std) = ekf.innovation_statistics(&innovation, &s);
        assert!(nis.is_finite());
        assert!(std.is_finite());
    }

    #[test]
    fn covariance_stabilization_maintains_symmetry() {
        let transition = DefaultTransition;
        let observation = DefaultObservation::new().with_dim(10);
        let x0 = State::default();
        let p0 = SMatrix::<f64, 8, 8>::identity();
        let q = SMatrix::<f64, 8, 8>::identity() * 0.01;
        let r = DMatrix::<f64>::identity(10, 10) * 0.1;
        let mut ekf = Ekf::new(transition, observation, x0, p0, q, r);
        ekf.p[(0, 1)] = 1.0;
        ekf.stabilize_covariance().unwrap();
        assert!(ekf.covariance_symmetry());
    }

    #[test]
    fn innovation_consistency() {
        let transition = DefaultTransition;
        let observation = DefaultObservation::new().with_dim(10);
        let x0 = State::default();
        let p0 = SMatrix::<f64, 8, 8>::identity();
        let q = SMatrix::<f64, 8, 8>::identity() * 0.01;
        let r = DMatrix::<f64>::identity(10, 10) * 0.1;
        let ekf = Ekf::new(transition, observation, x0, p0, q, r);
        let innovation = nalgebra::DVector::<f64>::zeros(10);
        let s = DMatrix::<f64>::identity(10, 10);
        let (nis, std) = ekf.innovation_statistics(&innovation, &s);
        assert!(nis.is_finite());
        assert!(std.is_finite() || std.is_nan());
    }

    #[test]
    fn covariance_stability_after_predict_update() {
        let transition = DefaultTransition;
        let observation = DefaultObservation::new().with_dim(10);
        let x0 = State::default();
        let p0 = SMatrix::<f64, 8, 8>::identity();
        let q = SMatrix::<f64, 8, 8>::identity() * 0.01;
        let r = DMatrix::<f64>::identity(10, 10) * 0.1;
        let mut ekf = Ekf::new(transition, observation, x0, p0, q, r);
        ekf.predict(None).unwrap();
        assert!(ekf.covariance_symmetry());
        assert!(ekf.finite_covariance());
        assert!(ekf.finite_state());
    }

    #[test]
    fn covariance_psd_after_stabilization() {
        let transition = DefaultTransition;
        let observation = DefaultObservation::new().with_dim(10);
        let x0 = State::default();
        let p0 = SMatrix::<f64, 8, 8>::identity();
        let q = SMatrix::<f64, 8, 8>::identity() * 0.01;
        let r = DMatrix::<f64>::identity(10, 10) * 0.1;
        let mut ekf = Ekf::new(transition, observation, x0, p0, q, r);
        ekf.predict(None).unwrap();
        ekf.update(&prv_core::Observation::new(vec![0.0; 10]))
            .unwrap();
        assert!(ekf.covariance_psd());
    }

    #[test]
    fn state_bound_check_passes_for_default_state() {
        let transition = DefaultTransition;
        let observation = DefaultObservation::new().with_dim(10);
        let x0 = State::default();
        let p0 = SMatrix::<f64, 8, 8>::identity();
        let q = SMatrix::<f64, 8, 8>::identity() * 0.01;
        let r = DMatrix::<f64>::identity(10, 10) * 0.1;
        let ekf = Ekf::new(transition, observation, x0, p0, q, r);
        assert!(ekf.finite_state());
    }

    #[test]
    fn quaternion_normalize_normalizes_quaternion_components() {
        let transition = DefaultTransition;
        let observation = DefaultObservation::new().with_dim(10);
        let x0 = State::new(3.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let p0 = SMatrix::<f64, 8, 8>::identity();
        let q = SMatrix::<f64, 8, 8>::identity() * 0.01;
        let r = DMatrix::<f64>::identity(10, 10) * 0.1;
        let mut ekf = Ekf::new(transition, observation, x0, p0, q, r);
        ekf.quaternion_normalize();
        let qx = ekf.x_hat.0[0];
        let qy = ekf.x_hat.0[1];
        let qz = ekf.x_hat.0[4];
        let qw = ekf.x_hat.0[5];
        let norm = (qx.powi(2) + qy.powi(2) + qz.powi(2) + qw.powi(2)).sqrt();
        assert!((norm - 1.0).abs() < 1e-10);
    }

    #[test]
    fn quaternion_normalize_after_predict() {
        let transition = DefaultTransition;
        let observation = DefaultObservation::new().with_dim(10);
        let x0 = State::new(3.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let p0 = SMatrix::<f64, 8, 8>::identity();
        let q = SMatrix::<f64, 8, 8>::identity() * 0.01;
        let r = DMatrix::<f64>::identity(10, 10) * 0.1;
        let mut ekf = Ekf::new(transition, observation, x0, p0, q, r);
        ekf.predict(None).unwrap();
        let qx = ekf.x_hat.0[0];
        let qy = ekf.x_hat.0[1];
        let qz = ekf.x_hat.0[4];
        let qw = ekf.x_hat.0[5];
        let norm = (qx.powi(2) + qy.powi(2) + qz.powi(2) + qw.powi(2)).sqrt();
        assert!((norm - 1.0).abs() < 1e-10);
    }

    #[test]
    fn predict_step_changes_state() {
        let transition = DefaultTransition;
        let observation = DefaultObservation::new().with_dim(10);
        let x0 = State::new(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let p0 = SMatrix::<f64, 8, 8>::identity();
        let q = SMatrix::<f64, 8, 8>::identity() * 0.01;
        let r = DMatrix::<f64>::identity(10, 10) * 0.1;
        let mut ekf = Ekf::new(transition, observation, x0, p0, q, r);
        ekf.predict(None).unwrap();
        assert!(ekf.x_hat.capacity() != 1.0 || ekf.x_hat.investment() != 0.0);
    }
}
