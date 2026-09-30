use serde::{Deserialize, Serialize};

use crate::{
    error::OptimizerError,
    objective::{Objective, Trajectory, WeightedObjective},
};

/// Per-period box bounds, `lower[i] <= u[i] <= upper[i]`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    lower: Vec<f64>,
    upper: Vec<f64>,
}

impl Bounds {
    /// Build bounds, rejecting `NaN` and any period where `lower > upper` (an
    /// empty box, i.e. no feasible value at all).
    ///
    /// Infinities are accepted: `±inf` is the encoding of "unbounded on this
    /// side", which is what [`Bounds::unbounded`] is built from. `NaN` is not,
    /// because clamping against `NaN` propagates it and a `NaN` bound silently
    /// destroys the projection.
    ///
    /// # Errors
    ///
    /// [`OptimizerError::LengthMismatch`] if the vectors differ in length,
    /// [`OptimizerError::EmptyTrajectory`] if both are empty,
    /// [`OptimizerError::NonFinite`] naming the first `NaN` bound, and
    /// [`OptimizerError::EmptyBounds`] naming the first period where
    /// `lower > upper`.
    pub fn new(lower: Vec<f64>, upper: Vec<f64>) -> Result<Self, OptimizerError> {
        if lower.len() != upper.len() {
            return Err(OptimizerError::LengthMismatch {
                bounds: lower.len(),
                trajectory: upper.len(),
            });
        }
        if lower.is_empty() {
            return Err(OptimizerError::EmptyTrajectory);
        }
        for (period, (&lo, &hi)) in lower.iter().zip(upper.iter()).enumerate() {
            if lo.is_nan() {
                return Err(OptimizerError::non_finite("lower bound", period));
            }
            if hi.is_nan() {
                return Err(OptimizerError::non_finite("upper bound", period));
            }
            if lo > hi {
                return Err(OptimizerError::EmptyBounds {
                    period,
                    lower: format!("{lo}"),
                    upper: format!("{hi}"),
                });
            }
        }
        Ok(Self { lower, upper })
    }

    /// Bounds that admit any finite value.
    ///
    /// # Errors
    ///
    /// [`OptimizerError::EmptyTrajectory`] if `n` is 0.
    pub fn unbounded(n: usize) -> Result<Self, OptimizerError> {
        Self::new(vec![f64::NEG_INFINITY; n], vec![f64::INFINITY; n])
    }

    /// Bounds fixed at a single value in every period.
    ///
    /// # Errors
    ///
    /// [`OptimizerError::EmptyTrajectory`] if `n` is 0, and
    /// [`OptimizerError::NonFinite`] if `value` is `NaN`.
    pub fn fixed(n: usize, value: f64) -> Result<Self, OptimizerError> {
        Self::new(vec![value; n], vec![value; n])
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.lower.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.lower.is_empty()
    }

    #[must_use]
    pub fn lower(&self) -> &[f64] {
        &self.lower
    }

    #[must_use]
    pub fn upper(&self) -> &[f64] {
        &self.upper
    }
}

/// A linear inequality constraint `coeffs . u <= rhs`.
///
/// Carries an `id` so an infeasibility report names *which* constraint
/// conflicted, rather than reporting a bare violation count.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinearConstraint {
    id: String,
    coeffs: Vec<f64>,
    rhs: f64,
}

impl LinearConstraint {
    /// Build a constraint.
    ///
    /// Coefficients must be finite — an infinite coefficient would make the
    /// projection arithmetic produce `NaN`. The right-hand side may be
    /// infinite, which is meaningful: `+inf` is a constraint that never binds,
    /// `-inf` is one that can never be satisfied.
    ///
    /// # Errors
    ///
    /// [`OptimizerError::EmptyTrajectory`] if `coeffs` is empty, and
    /// [`OptimizerError::NonFinite`] naming the first non-finite coefficient or
    /// a `NaN` right-hand side.
    pub fn new(id: impl Into<String>, coeffs: Vec<f64>, rhs: f64) -> Result<Self, OptimizerError> {
        if coeffs.is_empty() {
            return Err(OptimizerError::EmptyTrajectory);
        }
        if rhs.is_nan() {
            return Err(OptimizerError::non_finite("constraint rhs", 0));
        }
        for (index, &c) in coeffs.iter().enumerate() {
            if !c.is_finite() {
                return Err(OptimizerError::non_finite("constraint coefficient", index));
            }
        }
        Ok(Self {
            id: id.into(),
            coeffs,
            rhs,
        })
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.coeffs.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.coeffs.is_empty()
    }

    /// `coeffs . u`, the constraint's left-hand side.
    #[must_use]
    pub fn lhs(&self, u: &Trajectory) -> f64 {
        self.coeffs
            .iter()
            .zip(u.as_slice())
            .map(|(c, &v)| c * v)
            .sum()
    }

    /// How far `u` violates this constraint, `0.0` when satisfied.
    #[must_use]
    pub fn violation(&self, u: &Trajectory) -> f64 {
        (self.lhs(u) - self.rhs).max(0.0)
    }

    #[must_use]
    pub fn coeffs(&self) -> &[f64] {
        &self.coeffs
    }

    #[must_use]
    pub const fn rhs(&self) -> f64 {
        self.rhs
    }
}

/// A complete optimization problem: what to minimize, within what bounds, under
/// which constraints.
pub struct Problem<'a> {
    objective: &'a dyn Objective,
    bounds: Bounds,
    constraints: Vec<LinearConstraint>,
}

impl<'a> Problem<'a> {
    /// Assemble a problem, cross-checking that every length agrees.
    ///
    /// # Errors
    ///
    /// [`OptimizerError::ConstraintLength`] naming the offending constraint if
    /// its coefficient vector does not have one entry per period.
    pub fn new(
        objective: &'a dyn Objective,
        bounds: Bounds,
        constraints: Vec<LinearConstraint>,
    ) -> Result<Self, OptimizerError> {
        let n = bounds.len();
        for c in &constraints {
            if c.len() != n {
                return Err(OptimizerError::ConstraintLength {
                    id: c.id().to_owned(),
                    coeffs: c.len(),
                    trajectory: n,
                });
            }
        }
        Ok(Self {
            objective,
            bounds,
            constraints,
        })
    }

    /// Convenience constructor for the common "one weighted objective" case.
    ///
    /// # Errors
    ///
    /// [`OptimizerError::ConstraintLength`] if a constraint's length disagrees
    /// with the bounds.
    pub fn single(
        objective: &'a WeightedObjective<'a>,
        bounds: Bounds,
        constraints: Vec<LinearConstraint>,
    ) -> Result<Self, OptimizerError> {
        Self::new(objective, bounds, constraints)
    }

    #[must_use]
    pub const fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    #[must_use]
    pub fn constraints(&self) -> &[LinearConstraint] {
        &self.constraints
    }

    #[must_use]
    pub fn objective(&self) -> &'a dyn Objective {
        self.objective
    }
}
