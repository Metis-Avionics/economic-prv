use serde::{Deserialize, Serialize};

use crate::error::OptimizerError;

/// A decision variable sampled on a discrete grid of `n` periods.
///
/// Higher or lower is not interpreted here. The optimizer treats it as an
/// ordered vector; what a period's value *means* is the caller's business.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Trajectory {
    values: Vec<f64>,
}

impl Trajectory {
    /// Build a trajectory, rejecting an empty or non-finite one.
    ///
    /// # Errors
    ///
    /// [`OptimizerError::EmptyTrajectory`] if `values` is empty, and
    /// [`OptimizerError::NonFinite`] naming the first `NaN` or infinite value.
    pub fn new(values: Vec<f64>) -> Result<Self, OptimizerError> {
        if values.is_empty() {
            return Err(OptimizerError::EmptyTrajectory);
        }
        for (index, &v) in values.iter().enumerate() {
            if !v.is_finite() {
                return Err(OptimizerError::non_finite("trajectory value", index));
            }
        }
        Ok(Self { values })
    }

    /// A trajectory whose every period equals `value`.
    ///
    /// # Errors
    ///
    /// [`OptimizerError::EmptyTrajectory`] if `n` is 0, and
    /// [`OptimizerError::NonFinite`] if `value` is `NaN` or infinite.
    pub fn constant(n: usize, value: f64) -> Result<Self, OptimizerError> {
        Self::new(vec![value; n])
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.values.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[f64] {
        &self.values
    }

    pub(crate) fn values_mut(&mut self) -> &mut [f64] {
        &mut self.values
    }
}

/// A convex, differentiable objective supplied by the caller.
///
/// Implementors provide the value and its gradient. The optimizer never
/// differentiates anything itself, so it cannot disagree with the caller about
/// what the objective is.
pub trait Objective {
    /// The objective at `u`.
    ///
    /// # Errors
    ///
    /// Whatever the implementor needs to report; the optimizer propagates it
    /// unchanged rather than substituting a number.
    fn value(&self, u: &Trajectory) -> Result<f64, OptimizerError>;

    /// The gradient of [`Objective::value`] at `u`.
    ///
    /// # Errors
    ///
    /// Whatever the implementor needs to report. A gradient whose length
    /// differs from `u` is rejected rather than truncated.
    fn gradient(&self, u: &Trajectory) -> Result<Trajectory, OptimizerError>;
}

/// A weighted sum of objectives: `sum_i weight_i * term_i(u)`.
///
/// This is the shape of "a cost, a risk, and an implementation penalty", but
/// the crate does not name them — the caller chooses the terms and the weights,
/// and the weights are free to be zero, negative, or anything else.
pub struct WeightedObjective<'a> {
    terms: Vec<(f64, &'a dyn Objective)>,
}

impl<'a> WeightedObjective<'a> {
    #[must_use]
    pub fn new(terms: Vec<(f64, &'a dyn Objective)>) -> Self {
        Self { terms }
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.terms.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
}

impl Objective for WeightedObjective<'_> {
    fn value(&self, u: &Trajectory) -> Result<f64, OptimizerError> {
        let mut total = 0.0;
        for (weight, term) in &self.terms {
            total += weight * term.value(u)?;
        }
        Ok(total)
    }

    fn gradient(&self, u: &Trajectory) -> Result<Trajectory, OptimizerError> {
        let n = u.len();
        let mut total = vec![0.0; n];
        for (weight, term) in &self.terms {
            let g = term.gradient(u)?;
            if g.len() != n {
                return Err(OptimizerError::LengthMismatch {
                    bounds: n,
                    trajectory: g.len(),
                });
            }
            for (slot, component) in total.iter_mut().zip(g.as_slice()) {
                *slot += weight * component;
            }
        }
        Trajectory::new(total)
    }
}
