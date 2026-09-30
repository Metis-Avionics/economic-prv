//! Deterministic, constrained trajectory optimization.
//!
//! A small, domain-neutral optimizer for the shape of problem
//!
//! ```text
//! minimize  f(u)      subject to   lower <= u <= upper
//!                                  a . u <= b   for each linear constraint
//! ```
//! where `u` is a decision trajectory over `n` periods and `f` is supplied by
//! the caller.
//!
//! ## Why this exists, and what it deliberately does not know
//!
//! `prv-policy` scores macroeconomic *instruments* against a fixed 8-dimensional
//! state: its `PolicyInstrument` is a closed enum, its `PolicyWeights` is a
//! closed struct, and `evaluate` returns a static score map with no time
//! dimension. There is consequently no way to express "choose a deployment
//! schedule subject to a budget and a capacity ceiling", which is a very
//! common shape outside macroeconomics.
//!
//! So the machinery is here and the *semantics* are not. This crate has no
//! opinion about what is being scheduled. It does not know about markets,
//! policies, cyber risk, or control deployment. It takes an [`Objective`] from
//! the caller and finds a feasible minimizer of it. A consumer that wants
//! `cost + lambda * risk + mu * impact` builds that with [`WeightedObjective`]
//! and supplies the three terms.
//!
//! Keeping the domain out is the point. A general-purpose crate that hard-codes
//! one domain's concepts stops being reusable for the next one, and couples its
//! consumers to vocabulary they did not choose.
//!
//! ## Properties
//!
//! - **Deterministic.** No RNG anywhere. Identical inputs give bit-identical
//!   output, which is what makes a result reproducible without carrying a seed.
//! - **Bounded.** The iteration count is a configured maximum, never unbounded.
//! - **Fail-closed.** An infeasible constraint set is *reported*, with the
//!   offending constraint ids. It is never quietly relaxed, because a
//!   constraint that is silently dropped is a constraint nobody is relying on.
//! - **Non-convergence is a state, not a panic.** It is returned as
//!   [`OptimizationOutcome::NonConvergent`] with the last iterate, so a caller
//!   can decide whether to accept it.
//!
//! ## Method
//!
//! Projected gradient descent. Each step takes a gradient step and projects the
//! result back onto the feasible set: first onto the box, then cyclically onto
//! each linear half-space. Feasibility is then *verified* rather than assumed —
//! if any constraint is still violated beyond tolerance, the outcome is
//! [`OptimizationOutcome::Infeasible`].

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro,
    clippy::use_debug
)]

mod error;
mod objective;
mod problem;
mod project;
mod solve;

pub use error::OptimizerError;
pub use objective::{Objective, Trajectory, WeightedObjective};
pub use problem::{Bounds, LinearConstraint, Problem};
pub use solve::{OptimizationOutcome, SolverConfig, SolverReport, minimize};
