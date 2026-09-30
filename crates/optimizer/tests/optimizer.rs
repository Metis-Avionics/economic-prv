//! Behavioural tests for the optimizer.
//!
//! The properties asserted here are the ones a caller actually relies on:
//! the solution is feasible, it is the *right* feasible point, it is
//! reproducible, and the failure states are reported rather than hidden.

use prv_optimizer::*;

/// A separable convex quadratic: `sum_i w_i * (u_i - target_i)^2`.
///
/// The analytic minimizer over a box is `clamp(target_i, lo_i, hi_i)`, which
/// makes the expected answer checkable by hand rather than by comparison with
/// whatever the solver happened to produce.
struct Quadratic {
    weight: f64,
    target: Vec<f64>,
}

struct QuadraticGradient {
    weight: f64,
    target: Vec<f64>,
}

impl Objective for Quadratic {
    fn value(&self, u: &Trajectory) -> Result<f64, OptimizerError> {
        let mut total = 0.0;
        for (i, &v) in u.as_slice().iter().enumerate() {
            let d = v - self.target[i];
            total = (self.weight * d).mul_add(d, total);
        }
        Ok(total)
    }

    fn gradient(&self, u: &Trajectory) -> Result<Trajectory, OptimizerError> {
        let values: Vec<f64> = u
            .as_slice()
            .iter()
            .enumerate()
            .map(|(i, &v)| 2.0 * self.weight * (v - self.target[i]))
            .collect();
        Trajectory::new(values)
    }
}

impl Objective for QuadraticGradient {
    fn value(&self, _u: &Trajectory) -> Result<f64, OptimizerError> {
        Ok(0.0)
    }

    fn gradient(&self, u: &Trajectory) -> Result<Trajectory, OptimizerError> {
        let values: Vec<f64> = u
            .as_slice()
            .iter()
            .enumerate()
            .map(|(i, &v)| 2.0 * self.weight * (v - self.target[i]))
            .collect();
        Trajectory::new(values)
    }
}

/// Pulls the first period toward `target`. The simplest objective with a
/// non-zero curvature, which is what makes step-size behaviour observable.
struct Pull {
    target: f64,
}

impl Objective for Pull {
    fn value(&self, u: &Trajectory) -> Result<f64, OptimizerError> {
        Ok((u.as_slice()[0] - self.target).powi(2))
    }

    fn gradient(&self, u: &Trajectory) -> Result<Trajectory, OptimizerError> {
        Trajectory::new(vec![2.0 * (u.as_slice()[0] - self.target)])
    }
}

fn quadratic(weight: f64, target: &[f64]) -> Quadratic {
    Quadratic {
        weight,
        target: target.to_vec(),
    }
}

const fn config() -> SolverConfig {
    SolverConfig {
        max_iterations: 20_000,
        step_size: 0.05,
        tolerance: 1e-12,
        feasibility_tolerance: 1e-9,
        projection_sweeps: 8,
    }
}

fn close(a: &[f64], b: &[f64], tol: f64) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol)
}

// ---------------------------------------------------------------------------
// Feasibility
// ---------------------------------------------------------------------------

#[test]
fn unconstrained_solution_matches_the_analytic_minimizer() {
    let target = [1.0, -2.0, 3.5];
    let bounds = Bounds::unbounded(3).expect("valid bounds");
    let obj = quadratic(1.0, &target);
    let problem = Problem::new(&obj, bounds, vec![]).expect("valid problem");

    let (outcome, _) = minimize(&problem, &config()).expect("solver runs");
    let u = outcome.trajectory().expect("a trajectory exists");
    assert!(
        close(u.as_slice(), &target, 1e-6),
        "expected the analytic minimizer {target:?}, got {:?}",
        u.as_slice()
    );
    assert!(outcome.is_optimal(), "outcome was {outcome:?}");
}

#[test]
fn solution_respects_box_bounds() {
    // Target far above the ceiling, so the answer is pinned to the ceiling.
    let target = [100.0, 100.0];
    let bounds = Bounds::new(vec![-1.0, -1.0], vec![2.0, 3.0]).expect("valid bounds");
    let obj = quadratic(1.0, &target);
    let problem = Problem::new(&obj, bounds, vec![]).expect("valid problem");

    let (outcome, report) = minimize(&problem, &config()).expect("solver runs");
    let u = outcome.trajectory().expect("a trajectory exists");
    assert!(
        close(u.as_slice(), &[2.0, 3.0], 1e-6),
        "got {:?}",
        u.as_slice()
    );
    assert!(report.max_constraint_violation <= 1e-9);
}

#[test]
fn zero_width_bounds_are_a_valid_degenerate_case() {
    // Every period pinned: the feasible set is a single point.
    let bounds = Bounds::fixed(3, 7.0).expect("valid bounds");
    let obj = quadratic(1.0, &[0.0, 0.0, 0.0]);
    let problem = Problem::new(&obj, bounds, vec![]).expect("valid problem");

    let (outcome, _) = minimize(&problem, &config()).expect("solver runs");
    let u = outcome.trajectory().expect("a trajectory exists");
    assert!(
        close(u.as_slice(), &[7.0, 7.0, 7.0], 1e-9),
        "got {:?}",
        u.as_slice()
    );
    assert!(
        matches!(outcome, OptimizationOutcome::Optimal { .. }),
        "a singleton feasible set is still solvable, got {outcome:?}"
    );
}

#[test]
fn a_binding_linear_constraint_is_respected() {
    // sum(u) <= 2 with both periods wanting to be 10.
    let bounds = Bounds::new(vec![0.0, 0.0], vec![10.0, 10.0]).expect("valid bounds");
    let constraint = LinearConstraint::new("total", vec![1.0, 1.0], 2.0).expect("valid constraint");
    let obj = quadratic(1.0, &[10.0, 10.0]);
    let problem = Problem::new(&obj, bounds, vec![constraint]).expect("valid problem");

    let (outcome, report) = minimize(&problem, &config()).expect("solver runs");
    let u = outcome.trajectory().expect("a trajectory exists");
    let sum: f64 = u.as_slice().iter().sum();
    assert!(
        (sum - 2.0).abs() <= 1e-6,
        "constraint violated: sum = {sum}"
    );
    assert!(
        report.max_constraint_violation <= 1e-9,
        "report: {report:?}"
    );
}

// ---------------------------------------------------------------------------
// Infeasibility is a reported state
// ---------------------------------------------------------------------------

#[test]
fn infeasible_constraints_are_reported_with_their_ids() {
    // sum(u) <= 1 while each period is forced to at least 4: unsatisfiable.
    let bounds = Bounds::new(vec![4.0, 4.0], vec![10.0, 10.0]).expect("valid bounds");
    let conflict = LinearConstraint::new("total", vec![1.0, 1.0], 1.0).expect("valid");
    let obj = quadratic(1.0, &[4.0, 4.0]);
    let problem = Problem::new(&obj, bounds, vec![conflict]).expect("valid problem");

    let (outcome, report) = minimize(&problem, &config()).expect("solver runs");
    match &outcome {
        OptimizationOutcome::Infeasible { violated, .. } => {
            assert!(
                violated.contains(&"total".to_owned()),
                "named the wrong ids: {violated:?}"
            );
        }
        other => panic!("expected Infeasible, got {other:?}"),
    }
    assert!(report.max_constraint_violation > 0.0);
    assert!(
        outcome.trajectory().is_none(),
        "an infeasible problem must not hand back a trajectory that looks like an answer"
    );
}

#[test]
fn contradictory_constraints_are_reported_not_silently_relaxed() {
    // u <= 0 and -u <= -10, i.e. u <= 0 and u >= 10.
    let bounds = Bounds::unbounded(1).expect("valid bounds");
    let a = LinearConstraint::new("upper", vec![1.0], 0.0).expect("valid");
    let b = LinearConstraint::new("lower", vec![-1.0], -10.0).expect("valid");
    let obj = quadratic(1.0, &[5.0]);
    let problem = Problem::new(&obj, bounds, vec![a, b]).expect("valid problem");

    let (outcome, _) = minimize(&problem, &config()).expect("solver runs");
    assert!(
        !outcome.is_optimal(),
        "a contradictory constraint set must not report success"
    );
}

#[test]
fn a_constraint_with_no_normal_and_a_negative_rhs_is_infeasible() {
    // 0 * u <= -1 can never hold. It must be named, not skipped.
    let bounds = Bounds::unbounded(2).expect("valid bounds");
    let degenerate = LinearConstraint::new("degenerate", vec![0.0, 0.0], -1.0).expect("valid");
    let obj = quadratic(1.0, &[0.0, 0.0]);
    let problem = Problem::new(&obj, bounds, vec![degenerate]).expect("valid problem");

    let (outcome, _) = minimize(&problem, &config()).expect("solver runs");
    match &outcome {
        OptimizationOutcome::Infeasible { violated, .. } => assert!(
            violated.contains(&"degenerate".to_owned()),
            "an unsatisfiable constraint must be reported, got {violated:?}"
        ),
        other => panic!("expected Infeasible, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

#[test]
fn the_same_inputs_give_bit_identical_output() {
    // The objective outlives both Problems; only the problem is rebuilt, which
    // is exactly the "same inputs, run it again" situation being tested.
    let obj = quadratic(2.0, &[4.0, 3.0, 2.0]);
    let constraint = LinearConstraint::new("total", vec![1.0, 1.0, 1.0], 9.0).expect("constraint");

    let build = || {
        let bounds = Bounds::new(vec![0.0, 0.0, 0.0], vec![5.0, 5.0, 5.0]).expect("bounds");
        Problem::new(&obj, bounds, vec![constraint.clone()]).expect("problem")
    };

    let first = minimize(&build(), &config()).expect("solver runs").0;
    let second = minimize(&build(), &config()).expect("solver runs").0;

    assert_eq!(first, second, "the solver must be deterministic");
    if let (Some(a), Some(b)) = (first.trajectory(), second.trajectory()) {
        for (x, y) in a.as_slice().iter().zip(b.as_slice()) {
            assert_eq!(x.to_bits(), y.to_bits(), "bitwise mismatch: {x} vs {y}");
        }
    }
}

// ---------------------------------------------------------------------------
// Weighting: the property a "risk aversion" caller depends on
// ---------------------------------------------------------------------------

#[test]
fn raising_the_weight_on_a_term_moves_the_solution_toward_it() {
    let solve_with = |w_risk: f64| {
        let risk = Pull { target: 10.0 };
        let cost = Pull { target: 0.0 };
        let combined = WeightedObjective::new(vec![(1.0, &cost), (w_risk, &risk)]);
        let bounds = Bounds::unbounded(1).expect("bounds");
        let problem = Problem::new(&combined, bounds, vec![]).expect("problem");
        // Step size must respect the problem's curvature: the fixed point of
        // `u <- u - h * k * (u - target)` diverges when `h * k > 2`. With
        // k = 2 * (1 + w), w = 20 needs h < 2/42 ~= 0.048.
        let cfg = SolverConfig {
            step_size: 0.02,
            ..config()
        };
        let (outcome, _) = minimize(&problem, &cfg).expect("solver runs");
        outcome.trajectory().expect("trajectory").as_slice()[0]
    };

    let low = solve_with(0.0);
    let high = solve_with(20.0);

    assert!(
        low < 0.5,
        "with zero risk weight the cost term should win, got {low}"
    );
    assert!(
        high > 8.0,
        "with heavy risk weight the risk term should win, got {high}"
    );
    assert!(
        high > low,
        "increasing a term's weight must move the solution toward it: {low} -> {high}"
    );
}

/// A step size that violates the problem's curvature makes the iterate diverge.
/// That must surface as an error, not as a plausible-looking trajectory — the
/// documented precondition is `step_size < 2 / curvature`, and the failure mode
/// is loud rather than a wrong answer.
#[test]
fn a_diverging_step_size_is_reported_rather_than_returned_as_an_answer() {
    let bounds = Bounds::unbounded(1).expect("bounds");
    let obj = Pull { target: 10.0 };
    let problem = Problem::new(&obj, bounds, vec![]).expect("problem");

    let diverging = SolverConfig {
        step_size: 5.0,
        max_iterations: 200,
        ..config()
    };
    let result = minimize(&problem, &diverging);

    match result {
        Err(OptimizerError::NonFinite { .. }) => {}
        Err(other) => panic!("expected a non-finite report, got {other:?}"),
        Ok((outcome, _)) => {
            panic!("a diverging step size must not yield a usable outcome, got {outcome:?}")
        }
    }
}

// ---------------------------------------------------------------------------
// Non-convergence is a state
// ---------------------------------------------------------------------------

#[test]
fn a_tiny_iteration_budget_reports_non_convergence_rather_than_lying() {
    let bounds = Bounds::unbounded(3).expect("bounds");
    let obj = quadratic(1.0, &[9.0, 9.0, 9.0]);
    let problem = Problem::new(&obj, bounds, vec![]).expect("problem");

    let impatient = SolverConfig {
        max_iterations: 1,
        ..config()
    };
    let (outcome, report) = minimize(&problem, &impatient).expect("solver runs");

    assert_eq!(report.iterations, 1, "must stop at the configured ceiling");
    match &outcome {
        OptimizationOutcome::NonConvergent { trajectory, .. } => {
            assert!(
                !trajectory.as_slice().is_empty(),
                "the last iterate is returned"
            );
        }
        other => panic!("expected NonConvergent, got {other:?}"),
    }
}

#[test]
fn an_impossible_tolerance_does_not_stop_the_loop_early() {
    let bounds = Bounds::unbounded(2).expect("bounds");
    let obj = quadratic(1.0, &[5.0, 5.0]);
    let problem = Problem::new(&obj, bounds, vec![]).expect("problem");

    let strict = SolverConfig {
        max_iterations: 3_000,
        tolerance: 1e-300,
        ..config()
    };
    let (_, report) = minimize(&problem, &strict).expect("solver runs");
    assert_eq!(
        report.iterations, 3_000,
        "an unreachable tolerance must run to the ceiling, not exit early"
    );
}

// ---------------------------------------------------------------------------
// Input validation: the class, not the instance
// ---------------------------------------------------------------------------

#[test]
fn every_constructor_rejects_its_own_bad_input() {
    assert_eq!(
        Trajectory::new(vec![]),
        Err(OptimizerError::EmptyTrajectory)
    );
    assert!(matches!(
        Trajectory::new(vec![1.0, f64::NAN]),
        Err(OptimizerError::NonFinite { index: 1, .. })
    ));
    assert!(matches!(
        Bounds::new(vec![0.0], vec![0.0, 1.0]),
        Err(OptimizerError::LengthMismatch { .. })
    ));
    assert!(matches!(
        Bounds::new(vec![5.0], vec![1.0]),
        Err(OptimizerError::EmptyBounds { period: 0, .. })
    ));
    // NaN bounds are rejected; infinities are not, because they are how
    // "unbounded" is spelled.
    assert!(matches!(
        Bounds::new(vec![0.0], vec![f64::NAN]),
        Err(OptimizerError::NonFinite { .. })
    ));
    assert!(matches!(
        Bounds::new(vec![f64::NAN], vec![1.0]),
        Err(OptimizerError::NonFinite { .. })
    ));
    assert!(Bounds::unbounded(2).is_ok(), "infinite bounds are legal");
    assert!(matches!(
        LinearConstraint::new("c", vec![f64::INFINITY], 1.0),
        Err(OptimizerError::NonFinite { .. })
    ));
    assert!(matches!(
        LinearConstraint::new("c", vec![], 1.0),
        Err(OptimizerError::EmptyTrajectory)
    ));
    assert!(matches!(
        LinearConstraint::new("c", vec![1.0], f64::NAN),
        Err(OptimizerError::NonFinite { .. })
    ));
}

#[test]
fn a_constraint_of_the_wrong_length_is_rejected_at_assembly() {
    let bounds = Bounds::unbounded(3).expect("bounds");
    let wrong = LinearConstraint::new("short", vec![1.0], 1.0).expect("valid");
    let obj = quadratic(1.0, &[0.0; 3]);
    assert!(matches!(
        Problem::new(&obj, bounds, vec![wrong]),
        Err(OptimizerError::ConstraintLength { .. })
    ));
}

#[test]
fn every_solver_config_field_is_validated() {
    let bounds = Bounds::unbounded(1).expect("bounds");
    let obj = quadratic(1.0, &[0.0]);
    let problem = Problem::new(&obj, bounds, vec![]).expect("problem");

    let bad = [
        SolverConfig {
            max_iterations: 0,
            ..config()
        },
        SolverConfig {
            step_size: 0.0,
            ..config()
        },
        SolverConfig {
            step_size: f64::NAN,
            ..config()
        },
        SolverConfig {
            tolerance: -1.0,
            ..config()
        },
        SolverConfig {
            feasibility_tolerance: 0.0,
            ..config()
        },
        SolverConfig {
            projection_sweeps: 0,
            ..config()
        },
    ];
    for c in bad {
        assert!(
            matches!(
                minimize(&problem, &c),
                Err(OptimizerError::InvalidConfig(_))
            ),
            "config {c:?} should have been rejected"
        );
    }
}

#[test]
fn an_objective_that_returns_a_non_finite_gradient_is_reported() {
    struct Broken;
    impl Objective for Broken {
        fn value(&self, _u: &Trajectory) -> Result<f64, OptimizerError> {
            Ok(0.0)
        }
        fn gradient(&self, _u: &Trajectory) -> Result<Trajectory, OptimizerError> {
            Trajectory::new(vec![f64::NAN])
        }
    }
    let bounds = Bounds::unbounded(1).expect("bounds");
    let problem = Problem::new(&Broken, bounds, vec![]).expect("problem");
    assert!(minimize(&problem, &config()).is_err());
}

#[test]
fn an_objective_that_reports_a_different_length_is_rejected() {
    struct WrongLength;
    impl Objective for WrongLength {
        fn value(&self, _u: &Trajectory) -> Result<f64, OptimizerError> {
            Ok(0.0)
        }
        fn gradient(&self, _u: &Trajectory) -> Result<Trajectory, OptimizerError> {
            Trajectory::new(vec![1.0, 2.0])
        }
    }
    let bounds = Bounds::unbounded(3).expect("bounds");
    let problem = Problem::new(&WrongLength, bounds, vec![]).expect("problem");
    assert!(matches!(
        minimize(&problem, &config()),
        Err(OptimizerError::LengthMismatch { .. })
    ));
}

#[test]
fn weighted_objective_reports_its_own_size() {
    let a = QuadraticGradient {
        weight: 1.0,
        target: vec![0.0],
    };
    let b = QuadraticGradient {
        weight: 2.0,
        target: vec![1.0],
    };
    let combined = WeightedObjective::new(vec![(1.0, &a), (0.5, &b)]);
    assert_eq!(combined.len(), 2);
    assert!(!combined.is_empty());
    assert!(WeightedObjective::new(vec![]).is_empty());
}

/// A zero-weight term must not influence the gradient, or "switch this risk
/// off" would be a lie.
#[test]
fn a_zero_weight_term_contributes_nothing() {
    let active = Quadratic {
        weight: 1.0,
        target: vec![3.0],
    };
    let ignored = Quadratic {
        weight: 100.0,
        target: vec![-50.0],
    };
    let bounds = Bounds::unbounded(1).expect("bounds");

    let with_zero = WeightedObjective::new(vec![(1.0, &active), (0.0, &ignored)]);
    let p1 = Problem::new(&with_zero, bounds.clone(), vec![]).expect("problem");
    let a = minimize(&p1, &config()).expect("runs").0;

    let alone = WeightedObjective::new(vec![(1.0, &active)]);
    let p2 = Problem::new(&alone, bounds, vec![]).expect("problem");
    let b = minimize(&p2, &config()).expect("runs").0;

    assert_eq!(
        a.trajectory().map(Trajectory::as_slice),
        b.trajectory().map(Trajectory::as_slice),
        "a zero-weighted term must be exactly inert"
    );
}
