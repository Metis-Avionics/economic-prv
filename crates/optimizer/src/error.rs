use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Failure modes for the optimizer.
///
/// A dedicated type rather than a `prv_core::PrvError` variant: `PrvError`
/// derives `Eq`, so it cannot carry a float, and its existing variants are all
/// about state vectors and time series. Extending it would also be a breaking
/// change to `prv-core`'s public API, which downstream workspaces pin by minor
/// version.
#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizerError {
    /// A trajectory must have at least one period.
    #[error("a trajectory needs at least one period, got 0")]
    EmptyTrajectory,

    /// Bounds and trajectory disagree on length.
    #[error("bounds have {bounds} period(s) but the trajectory has {trajectory}")]
    LengthMismatch { bounds: usize, trajectory: usize },

    /// A constraint's coefficient vector does not match the trajectory.
    #[error("constraint '{id}' has {coeffs} coefficient(s) but the trajectory has {trajectory}")]
    ConstraintLength {
        id: String,
        coeffs: usize,
        trajectory: usize,
    },

    /// `lower > upper` for some period, so the box is empty.
    #[error("bounds are empty at period {period}: lower {lower} > upper {upper}")]
    EmptyBounds {
        period: usize,
        lower: String,
        upper: String,
    },

    /// A value that must be a usable number was not.
    ///
    /// `NaN` is rejected everywhere. Infinities are rejected only where they
    /// are meaningless: a bound or a right-hand side may legitimately be
    /// infinite (`Bounds::unbounded` is built from `±inf`, and a `+inf` rhs
    /// means a constraint that never binds), but an infinite *coefficient*
    /// would make the projection arithmetic produce `NaN`.
    #[error("{field} at index {index} is not a usable number (NaN or infinite)")]
    NonFinite { field: &'static str, index: usize },

    /// The solver configuration is not usable.
    #[error("solver configuration is invalid: {0}")]
    InvalidConfig(String),
}

impl OptimizerError {
    #[must_use]
    pub const fn non_finite(field: &'static str, index: usize) -> Self {
        Self::NonFinite { field, index }
    }
}
