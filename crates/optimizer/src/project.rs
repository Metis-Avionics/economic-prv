use crate::{Bounds, LinearConstraint, Trajectory};

/// Project `u` onto the box `lower <= u <= upper`, in place.
pub fn project_box(u: &mut Trajectory, bounds: &Bounds) {
    let values = u.values_mut();
    for (i, v) in values.iter_mut().enumerate() {
        *v = v.clamp(bounds.lower()[i], bounds.upper()[i]);
    }
}

/// Project `u` onto one half-space `coeffs . u <= rhs`, in place.
///
/// A single projection onto a half-space has a closed form: if the point
/// violates the constraint, move it along the constraint normal by exactly the
/// amount needed to reach the boundary. When the normal is degenerate (all
/// coefficients zero) the constraint is either always satisfied or never
/// satisfiable; that is reported by the caller as infeasible rather than
/// silently ignored here.
pub fn project_halfspace(u: &mut Trajectory, constraint: &LinearConstraint) {
    let norm_sq: f64 = constraint
        .coeffs()
        .iter()
        .fold(0.0, |acc, &c| c.mul_add(c, acc));
    if norm_sq <= f64::EPSILON {
        return;
    }
    let excess = constraint.lhs(u) - constraint.rhs();
    if excess <= 0.0 {
        return;
    }
    let scale = excess / norm_sq;
    let values = u.values_mut();
    for (v, &c) in values.iter_mut().zip(constraint.coeffs()) {
        *v = scale.mul_add(-c, *v);
    }
}

/// Names of constraints still violated by more than `tolerance`.
///
/// A constraint with an all-zero normal is reported regardless of tolerance:
/// `0 * u <= rhs` is unsatisfiable for any negative `rhs`, and there is no
/// projection that can fix it, so it must not be passed over in silence.
pub fn violated_ids(
    constraints: &[LinearConstraint],
    u: &Trajectory,
    tolerance: f64,
) -> Vec<String> {
    let mut ids = Vec::new();
    for c in constraints {
        let norm_sq: f64 = c.coeffs().iter().fold(0.0, |acc, &x| x.mul_add(x, acc));
        if norm_sq <= f64::EPSILON {
            if c.rhs() < 0.0 {
                ids.push(c.id().to_owned());
            }
            continue;
        }
        if c.violation(u) > tolerance {
            ids.push(c.id().to_owned());
        }
    }
    ids
}

/// Largest constraint violation in `u`, for reporting and for tests.
pub fn max_violation(constraints: &[LinearConstraint], u: &Trajectory) -> f64 {
    constraints
        .iter()
        .map(|c| c.violation(u))
        .fold(0.0_f64, f64::max)
}
