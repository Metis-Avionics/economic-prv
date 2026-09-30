use serde::{Deserialize, Serialize};

use crate::{
    Problem,
    error::OptimizerError,
    objective::Trajectory,
    project::{max_violation, project_box, project_halfspace, violated_ids},
};

/// Solver controls. Every one of them is bounded; none of them can make the
/// loop unbounded.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SolverConfig {
    /// Hard iteration ceiling. The loop cannot run longer than this.
    pub max_iterations: usize,
    /// Gradient step size.
    pub step_size: f64,
    /// Convergence tolerance on the projected gradient norm.
    pub tolerance: f64,
    /// Feasibility tolerance for constraint violation.
    pub feasibility_tolerance: f64,
    /// How many cyclic projection sweeps onto the half-spaces per iteration.
    pub projection_sweeps: usize,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            max_iterations: 1_000,
            step_size: 0.1,
            tolerance: 1e-9,
            feasibility_tolerance: 1e-9,
            projection_sweeps: 4,
        }
    }
}

impl SolverConfig {
    fn validate(&self) -> Result<(), OptimizerError> {
        let problems: Vec<String> = [
            (
                self.max_iterations == 0,
                "max_iterations must be at least 1",
            ),
            (
                !(self.step_size.is_finite() && self.step_size > 0.0),
                "step_size must be finite and positive",
            ),
            (
                !(self.tolerance.is_finite() && self.tolerance > 0.0),
                "tolerance must be finite and positive",
            ),
            (
                !(self.feasibility_tolerance.is_finite() && self.feasibility_tolerance > 0.0),
                "feasibility_tolerance must be finite and positive",
            ),
            (
                self.projection_sweeps == 0,
                "projection_sweeps must be at least 1",
            ),
        ]
        .iter()
        .filter_map(|(bad, msg)| if *bad { Some((*msg).to_owned()) } else { None })
        .collect();

        if problems.is_empty() {
            Ok(())
        } else {
            Err(OptimizerError::InvalidConfig(problems.join("; ")))
        }
    }
}

/// What the solver found.
///
/// Infeasibility and non-convergence are returned, not panicked on and not
/// papered over. A caller that receives [`OptimizationOutcome::Infeasible`]
/// learns which constraints conflicted; one that receives
/// [`OptimizationOutcome::NonConvergent`] learns the iteration count and can
/// inspect the last iterate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OptimizationOutcome {
    /// Converged to a point satisfying the bounds and every constraint.
    Optimal {
        trajectory: Trajectory,
        objective: f64,
        iterations: usize,
        projected_gradient_norm: f64,
    },
    /// The constraint set admits no feasible point. The ids name the
    /// constraints still violated after projection.
    Infeasible {
        violated: Vec<String>,
        max_violation: f64,
    },
    /// Ran out of iterations before meeting the tolerance. The last iterate is
    /// returned so a caller can decide whether to accept it.
    NonConvergent {
        trajectory: Trajectory,
        objective: f64,
        iterations: usize,
        projected_gradient_norm: f64,
    },
}

impl OptimizationOutcome {
    /// The trajectory, when one exists. `None` for an infeasible problem,
    /// which has no answer to give.
    #[must_use]
    pub const fn trajectory(&self) -> Option<&Trajectory> {
        match self {
            Self::Optimal { trajectory, .. } | Self::NonConvergent { trajectory, .. } => {
                Some(trajectory)
            }
            Self::Infeasible { .. } => None,
        }
    }

    #[must_use]
    pub const fn is_optimal(&self) -> bool {
        matches!(self, Self::Optimal { .. })
    }
}

/// How far the solver actually got, independent of the outcome.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SolverReport {
    pub iterations: usize,
    pub projected_gradient_norm: f64,
    pub max_constraint_violation: f64,
}

/// Minimize `problem`'s objective over its feasible set.
///
/// Projected gradient descent. Deterministic: there is no RNG, so identical
/// inputs give bit-identical output and a result needs no seed to reproduce.
///
/// # Errors
///
/// [`OptimizerError::InvalidConfig`] if any [`SolverConfig`] field is
/// unusable, [`OptimizerError::NonFinite`] if the objective reports a
/// non-finite value or the iterate diverges, and whatever the caller's
/// [`Objective`] reports. A mis-sized gradient is rejected rather than
/// truncated.
pub fn minimize(
    problem: &Problem<'_>,
    config: &SolverConfig,
) -> Result<(OptimizationOutcome, SolverReport), OptimizerError> {
    config.validate()?;

    let mut u = starting_point(problem)?;
    let mut iterations = 0usize;
    let mut gradient_norm = f64::INFINITY;

    while iterations < config.max_iterations {
        iterations += 1;
        let previous = u.clone();

        let gradient = problem.objective().gradient(&u)?;
        if gradient.len() != u.len() {
            // Without this the `zip` in `step_and_project` silently truncates a
            // mis-sized gradient, and the solver reports a confident answer
            // derived from a gradient the caller did not intend.
            return Err(OptimizerError::LengthMismatch {
                bounds: u.len(),
                trajectory: gradient.len(),
            });
        }
        step_and_project(&mut u, &gradient, problem, config);
        gradient_norm = projected_gradient_norm(&previous, &u, problem);

        if gradient_norm <= config.tolerance {
            break;
        }
    }

    let violated = violated_ids(problem.constraints(), &u, config.feasibility_tolerance);
    let report = SolverReport {
        iterations,
        projected_gradient_norm: gradient_norm,
        max_constraint_violation: max_violation(problem.constraints(), &u),
    };

    if !violated.is_empty() {
        let outcome = OptimizationOutcome::Infeasible {
            violated,
            max_violation: report.max_constraint_violation,
        };
        return Ok((outcome, report));
    }

    let objective = problem.objective().value(&u)?;
    if !objective.is_finite() {
        // A divergent iterate can still be *finite* (here: -7e191) while the
        // objective it produces overflows to infinity. Returning that as a
        // `NonConvergent` result would hand the caller a trajectory that looks
        // like an answer, so the divergence is reported instead.
        return Err(OptimizerError::non_finite("objective value", 0));
    }
    let converged = gradient_norm.is_finite() && gradient_norm <= config.tolerance;

    let outcome = if converged {
        OptimizationOutcome::Optimal {
            trajectory: u,
            objective,
            iterations,
            projected_gradient_norm: gradient_norm,
        }
    } else {
        OptimizationOutcome::NonConvergent {
            trajectory: u,
            objective,
            iterations,
            projected_gradient_norm: gradient_norm,
        }
    };
    Ok((outcome, report))
}

/// The midpoint of the box, which is always box-feasible.
fn starting_point(problem: &Problem<'_>) -> Result<Trajectory, OptimizerError> {
    let bounds = problem.bounds();
    let values: Vec<f64> = bounds
        .lower()
        .iter()
        .zip(bounds.upper())
        .map(|(lo, hi)| {
            if lo.is_finite() && hi.is_finite() {
                lo + (hi - lo) / 2.0
            } else if hi.is_finite() {
                hi - 1.0
            } else if lo.is_finite() {
                lo + 1.0
            } else {
                0.0
            }
        })
        .collect();
    Trajectory::new(values)
}

fn step_and_project(
    u: &mut Trajectory,
    gradient: &Trajectory,
    problem: &Problem<'_>,
    config: &SolverConfig,
) {
    let values = u.values_mut();
    for (v, &g) in values.iter_mut().zip(gradient.as_slice()) {
        *v = config.step_size.mul_add(-g, *v);
    }
    project_box(u, problem.bounds());
    for _ in 0..config.projection_sweeps {
        for c in problem.constraints() {
            project_halfspace(u, c);
        }
        project_box(u, problem.bounds());
    }
}

/// Distance the iterate moved, projected onto the feasible set.
///
/// The projected step is the standard stationarity measure for projected
/// gradient methods: a small projected step means the gradient is balanced
/// against the active constraints, not merely small.
fn projected_gradient_norm(
    previous: &Trajectory,
    current: &Trajectory,
    problem: &Problem<'_>,
) -> f64 {
    let mut probe = previous.clone();
    if let Ok(gradient) = problem.objective().gradient(previous) {
        let values = probe.values_mut();
        for (v, &g) in values.iter_mut().zip(gradient.as_slice()) {
            *v -= g;
        }
        project_box(&mut probe, problem.bounds());
        for _ in 0..4 {
            for c in problem.constraints() {
                project_halfspace(&mut probe, c);
            }
            project_box(&mut probe, problem.bounds());
        }
    }
    squared_distance(previous, &probe)
        .min(squared_distance(current, &probe))
        .sqrt()
}

fn squared_distance(a: &Trajectory, b: &Trajectory) -> f64 {
    a.as_slice()
        .iter()
        .zip(b.as_slice())
        .map(|(x, &y)| (x - y) * (x - y))
        .sum()
}
