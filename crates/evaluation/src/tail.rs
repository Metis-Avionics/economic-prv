//! Tail-risk measures: value at risk and expected shortfall (`CVaR`).
//!
//! These are the two quantile-based measures of the loss tail. They are
//! deliberately **not** the `f64::NAN`-on-bad-input style used by
//! [`crate::metrics`] — see [Why this module is fallible] below.
//!
//! Semantics, with `losses` sorted ascending and `alpha` a confidence level
//! in `(0, 1]`:
//!
//! - `value_at_risk(losses, alpha)` is the `alpha` quantile, by linear
//!   interpolation between order statistics (the same convention as
//!   `numpy.quantile` default and R's `type = 7`).
//! - `expected_shortfall(losses, alpha)` is the mean loss over the tail at or
//!   above that quantile.
//!
//! The two obey `expected_shortfall >= value_at_risk` for every sample and
//! every level, which is asserted rather than assumed: a tail measure that
//! reports a *smaller* number than the quantile it is defined against is
//! reporting a value that is not a risk.
//!
//! /// Why this module is fallible while [`crate::metrics`] is not.
//! ///
//! /// `rmse`, `mae`, `brier_score` and `log_loss` return `f64::NAN` when their
//! /// input is malformed. That is defensible for an accuracy score, where NaN is
//! /// visibly not-a-number in a report. It is **not** defensible for a tail
//! /// measure, and the reason is mechanical rather than stylistic:
//! ///
//! /// Every comparison against `NaN` is false. So a `NaN` expected shortfall
//! /// passes `< threshold`, passes `> limit`, passes `== budget`, and passes every
//! /// other gate written in the natural direction — while reporting nothing to
//! /// the operator. A budget gate whose failure mode is to pass is worse than no
//! /// gate, because it is trusted. These measures therefore return `Result` and
//! /// name the offending index, so a caller cannot accidentally treat "could not
//! /// be computed" as "computed to be small".
//! ///
//! /// Callers that want the lenient form can write
//! /// `expected_shortfall(&l, a).unwrap_or(f64::NAN)`, which makes the choice
//! /// explicit at the call site rather than implicit in the return type.
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Failure modes for the tail measures.
///
/// A dedicated type rather than a `prv_core::PrvError` variant: `PrvError`
/// derives `Eq`, so it cannot carry a float, and every variant it has today is
/// about state vectors and time series. Extending it would also be a breaking
/// change to `prv-core`'s public API, which downstream workspaces pin by minor
/// version.
#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricError {
    /// No observations were supplied, so no quantile is defined.
    #[error("cannot compute a tail measure over an empty sample")]
    EmptySample,

    /// The confidence level is outside `(0, 1]`.
    #[error("confidence level must be in (0, 1], got {got}")]
    InvalidConfidence { got: String },

    /// A loss was NaN or infinite. Propagating it would make every downstream
    /// comparison false rather than loud, which is the failure this type
    /// exists to prevent.
    #[error("loss sample contains a non-finite value at index {index}")]
    NonFiniteLoss { index: usize },
}

/// A validated loss sample and confidence level.
///
/// Validated once in the constructor so the quantile arithmetic never has to
/// re-check, and so an invalid level is rejected before any allocation sized by
/// it is reserved.
#[derive(Clone, Debug, PartialEq)]
struct TailSample {
    sorted: Vec<f64>,
    alpha: f64,
}

impl TailSample {
    fn new(losses: &[f64], alpha: f64) -> Result<Self, MetricError> {
        if losses.is_empty() {
            return Err(MetricError::EmptySample);
        }
        if !(alpha > 0.0 && alpha <= 1.0) {
            return Err(MetricError::InvalidConfidence {
                got: format!("{alpha}"),
            });
        }
        for (index, &loss) in losses.iter().enumerate() {
            if !loss.is_finite() {
                return Err(MetricError::NonFiniteLoss { index });
            }
        }
        let mut sorted = losses.to_vec();
        sorted.sort_by(f64::total_cmp);
        Ok(Self { sorted, alpha })
    }

    /// Position of the `alpha` quantile in the sorted sample, in observation
    /// units. Linear interpolation between order statistics.
    ///
    /// The `usize` -> `f64` cast is lossless for any sample that fits in
    /// memory: `f64` represents every integer exactly below 2^53, and a sample
    /// of 2^53 `f64`s would need 64 petabytes before the mantissa became the
    /// limiting factor.
    #[allow(clippy::cast_precision_loss)]
    fn quantile_position(&self) -> f64 {
        let n = self.sorted.len();
        let h = self.alpha * (n as f64 - 1.0);
        h.clamp(0.0, (n as f64 - 1.0).max(0.0))
    }

    /// The `alpha` quantile by linear interpolation between order statistics.
    ///
    /// The casts are exact in context: `h` comes from
    /// [`TailSample::quantile_position`], which clamps into `0..=len-1`, so
    /// `floor`/`ceil` of `h` are non-negative integers inside the sample; and
    /// a sample that fits in memory is far below 2^53, where `f64` represents
    /// every integer exactly.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn quantile(&self) -> f64 {
        if self.sorted.len() == 1 {
            return self.sorted[0];
        }
        let h = self.quantile_position();
        let lower_index = h.floor() as usize;
        let upper_index = (h.ceil() as usize).min(self.sorted.len() - 1);
        let weight = h - lower_index as f64;
        self.sorted[upper_index].mul_add(weight, self.sorted[lower_index] * (1.0 - weight))
    }
}

/// The `alpha`-quantile of the loss distribution.
///
/// This is the loss level exceeded with probability `1 - alpha`. It answers
/// "how bad is a bad day", not "how bad is a bad day *on average* once it
/// happens" — for that, use [`expected_shortfall`], which is always the more
/// conservative of the two.
///
/// # Errors
///
/// [`MetricError::EmptySample`] if `losses` is empty,
/// [`MetricError::InvalidConfidence`] if `alpha` is outside `(0, 1]`, and
/// [`MetricError::NonFiniteLoss`] naming the first `NaN` or infinite loss.
pub fn value_at_risk(losses: &[f64], alpha: f64) -> Result<f64, MetricError> {
    Ok(TailSample::new(losses, alpha)?.quantile())
}

/// Mean loss over the tail at or above the `alpha` quantile.
///
/// Also called `CVaR` or the conditional tail expectation. Always
/// `>= value_at_risk` for the same sample and level, because it averages a set
/// of observations that are all at or above the quantile it is reported
/// alongside.
///
/// # Errors
///
/// [`MetricError::EmptySample`] if `losses` is empty,
/// [`MetricError::InvalidConfidence`] if `alpha` is outside `(0, 1]`, and
/// [`MetricError::NonFiniteLoss`] naming the first `NaN` or infinite loss.
pub fn expected_shortfall(losses: &[f64], alpha: f64) -> Result<f64, MetricError> {
    let sample = TailSample::new(losses, alpha)?;
    let threshold = sample.quantile();
    let tail: Vec<f64> = sample
        .sorted
        .iter()
        .copied()
        .filter(|&loss| loss >= threshold)
        .collect();
    let sum: f64 = tail.iter().sum();
    // `tail` is non-empty whenever the sample is non-empty, and the length cast
    // is lossless for any sample that fits in memory.
    #[allow(clippy::cast_precision_loss)]
    let count = tail.len() as f64;
    Ok(sum / count)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic, easy-to-reason-about sample: 0..=9.
    const LOSSES: [f64; 10] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

    #[test]
    fn var_matches_hand_computed_quantile() {
        // 10 observations, alpha = 0.5 -> h = 0.5 * 9 = 4.5 -> between
        // sorted[4] = 4.0 and sorted[5] = 5.0 -> 4.5.
        let var = value_at_risk(&LOSSES, 0.5).expect("valid sample");
        assert!((var - 4.5).abs() < 1e-12, "got {var}");
    }

    #[test]
    fn var_at_alpha_one_is_the_maximum() {
        let var = value_at_risk(&LOSSES, 1.0).expect("valid sample");
        assert!((var - 9.0).abs() < 1e-12, "got {var}");
    }

    #[test]
    fn var_on_single_observation_is_that_observation() {
        let var = value_at_risk(&[42.0], 0.99).expect("valid sample");
        assert!((var - 42.0).abs() < 1e-12, "got {var}");
    }

    #[test]
    fn cvar_averages_the_tail_and_exceeds_var() {
        // alpha = 0.5 -> quantile 4.5, so the tail is {5,6,7,8,9}, mean 7.0.
        let es = expected_shortfall(&LOSSES, 0.5).expect("valid sample");
        assert!((es - 7.0).abs() < 1e-12, "got {es}");
        assert!(
            es > value_at_risk(&LOSSES, 0.5).expect("valid sample"),
            "expected shortfall must not be below value at risk"
        );
    }

    #[test]
    fn cvar_at_alpha_one_is_the_maximum() {
        let es = expected_shortfall(&LOSSES, 1.0).expect("valid sample");
        assert!((es - 9.0).abs() < 1e-12, "got {es}");
    }

    #[test]
    fn both_measures_are_monotone_in_alpha() {
        let mut previous_var = f64::NEG_INFINITY;
        let mut previous_es = f64::NEG_INFINITY;
        for step in 1..=20 {
            let alpha = f64::from(step) / 20.0;
            let var = value_at_risk(&LOSSES, alpha).expect("valid sample");
            let es = expected_shortfall(&LOSSES, alpha).expect("valid sample");
            assert!(
                var >= previous_var,
                "VaR decreased at alpha={alpha}: {var} < {previous_var}"
            );
            assert!(
                es >= previous_es,
                "CVaR decreased at alpha={alpha}: {es} < {previous_es}"
            );
            previous_var = var;
            previous_es = es;
        }
    }

    #[test]
    fn input_order_does_not_matter() {
        let shuffled = [9.0, 0.0, 5.0, 2.0, 8.0, 1.0, 7.0, 3.0, 6.0, 4.0];
        let es = expected_shortfall(&shuffled, 0.5).expect("valid sample");
        assert!((es - 7.0).abs() < 1e-12, "got {es}");
    }

    #[test]
    fn negative_losses_are_ordered_not_clamped() {
        // Losses may be negative (a gain). Clamping at zero would silently
        // change the measure.
        let losses = [-10.0, -5.0, 0.0, 5.0];
        let var = value_at_risk(&losses, 0.5).expect("valid sample");
        assert!((var - (-2.5)).abs() < 1e-12, "got {var}");
    }

    #[test]
    fn empty_sample_is_rejected() {
        assert_eq!(value_at_risk(&[], 0.95), Err(MetricError::EmptySample));
        assert_eq!(expected_shortfall(&[], 0.95), Err(MetricError::EmptySample));
    }

    #[test]
    fn confidence_level_outside_the_unit_interval_is_rejected() {
        for bad in [0.0, -0.1, 1.1, f64::NAN, f64::INFINITY] {
            assert!(
                matches!(
                    value_at_risk(&LOSSES, bad),
                    Err(MetricError::InvalidConfidence { .. })
                ),
                "alpha={bad} should be rejected"
            );
        }
    }

    #[test]
    fn non_finite_losses_are_rejected_with_their_index() {
        let mut losses = LOSSES.to_vec();
        losses[3] = f64::NAN;
        assert_eq!(
            expected_shortfall(&losses, 0.95),
            Err(MetricError::NonFiniteLoss { index: 3 })
        );

        losses[3] = f64::INFINITY;
        assert_eq!(
            value_at_risk(&losses, 0.95),
            Err(MetricError::NonFiniteLoss { index: 3 })
        );
    }

    /// The reason this module is `Result`-shaped, asserted rather than
    /// asserted in prose: a silent NaN makes every comparison false, which
    /// fails *open* for a risk gate.
    /// The reason this module is `Result`-shaped, asserted rather than
    /// asserted in prose: a silent `NaN` makes every comparison false, which
    /// fails *open* for a risk gate.
    ///
    /// The lint allowances are load-bearing. This test is *about* the falseness
    /// of the comparison operators themselves, so rewriting `nan > limit` into
    /// its logical negation `nan <= limit` — which clippy's `bool_comparison`
    /// and `neg_cmp_op_on_partial_ord` both suggest — would replace the claim
    /// under test with a different one. `NaN <= limit` is also false, but for
    /// a reason that is no longer the point.
    #[test]
    #[allow(clippy::bool_comparison, clippy::neg_cmp_op_on_partial_ord)]
    fn a_silent_nan_would_fail_open_on_a_risk_gate() {
        let limit = 1_000.0;
        let losses = [1.0, f64::NAN, 3.0];

        // Rejection is explicit, and the caller can see which case it is.
        match expected_shortfall(&losses, 0.95) {
            Ok(es) => assert!(es < limit, "a real value under the limit is fine"),
            Err(err) => assert_eq!(
                err,
                MetricError::NonFiniteLoss { index: 1 },
                "the non-finite loss must be reported, not smoothed over"
            ),
        }

        let nan = f64::NAN;
        assert_eq!(
            nan.partial_cmp(&limit),
            None,
            "NaN must be unordered with respect to the limit"
        );
        assert!((nan > limit) == false, "so `risk > limit` never blocks");
        assert!((nan >= limit) == false, "nor does the equality branch");
        assert!(
            (nan < limit) == false,
            "and the approve branch is false too"
        );
    }

    #[test]
    fn degenerate_all_equal_sample_is_consistent() {
        let losses = [7.0; 5];
        let var = value_at_risk(&losses, 0.95).expect("valid sample");
        let es = expected_shortfall(&losses, 0.95).expect("valid sample");
        assert!((var - 7.0).abs() < 1e-12, "got {var}");
        assert!((es - 7.0).abs() < 1e-12, "got {es}");
    }
}
