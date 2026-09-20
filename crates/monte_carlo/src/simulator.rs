use crate::{ShockSpec, State};
use nalgebra::{Cholesky, Const, SMatrix};
use rand::prelude::*;
use rand_distr::StandardNormal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use prv_core::PrvError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonteCarloProbabilities {
    pub capacity_increase: f64,
    pub capacity_decline: f64,
    pub investment_increase: f64,
    pub investment_decline: f64,
    pub regime_transition: HashMap<prv_core::Regime, f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonteCarloResults {
    pub mean: State,
    pub median: State,
    pub quantiles: Vec<(f64, State)>,
    pub probabilities: MonteCarloProbabilities,
    pub tail_risk: f64,
    pub regime_distribution: HashMap<prv_core::Regime, usize>,
    pub seed: u64,
    pub sample_count: usize,
}

#[derive(Clone, Debug)]
pub struct Simulator {
    pub seed: u64,
}

impl Simulator {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { seed }
    }

    #[must_use = "sampled states must be used"]
    /// Samples states from a multivariate normal distribution.
    ///
    /// # Errors
    ///
    /// Returns `PrvError::NonPsdCovariance` if Cholesky decomposition fails even after eigenvalue clipping.
    #[allow(clippy::unused_self)]
    pub fn sample_state(
        &self,
        mean: &State,
        covariance: &SMatrix<f64, 8, 8>,
        n: usize,
    ) -> Result<Vec<State>, PrvError> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed);
        let mut samples = Vec::with_capacity(n);

        let chol = self.prepare_cholesky(covariance)?;

        let l = chol.l();
        for _ in 0..n {
            let standard_normal: nalgebra::SVector<f64, 8> =
                nalgebra::SVector::from_fn(|_, _| rng.sample(StandardNormal));
            let mut sample_vec = *mean.as_vector();
            for i in 0..8 {
                let mut sum = 0.0;
                for j in 0..8 {
                    sum = l[(i, j)].mul_add(standard_normal[j], sum);
                }
                sample_vec[i] += sum;
            }
            samples.push(State(sample_vec));
        }

        Ok(samples)
    }

    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn run_path(&self, initial: &State, horizon: usize, shocks: &[ShockSpec]) -> Vec<State> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed);
        let mut path = vec![initial.clone()];
        let mut state = initial.clone();

        for _ in 0..horizon {
            for shock in shocks {
                shock.apply(&mut state, &mut rng);
            }
            path.push(state.clone());
        }

        path
    }

    #[must_use = "simulation results must be used"]
    #[allow(
        clippy::missing_panics_doc,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    /// Runs the Monte Carlo simulation.
    ///
    /// # Errors
    ///
    /// Returns `PrvError::NonPsdCovariance` if Cholesky decomposition fails.
    pub fn simulate(
        &self,
        mean: &State,
        covariance: &SMatrix<f64, 8, 8>,
        n_paths: usize,
        horizon: usize,
        shocks: &[ShockSpec],
    ) -> Result<MonteCarloResults, PrvError> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed);
        let chol = self.prepare_cholesky(covariance)?;
        let l = chol.l();
        let all_paths = self.run_paths(mean, &l, horizon, shocks, n_paths, &mut rng);
        self.compute_results(all_paths, n_paths, horizon)
    }

    #[allow(clippy::unused_self)]
    fn prepare_cholesky(
        &self,
        covariance: &SMatrix<f64, 8, 8>,
    ) -> Result<Cholesky<f64, Const<8>>, PrvError> {
        covariance.cholesky().map_or_else(
            || {
                let mut cov = *covariance;
                let eigen = cov.symmetric_eigenvalues();
                let min_eig = eigen.min();
                if min_eig < 1e-10 {
                    for i in 0..8 {
                        cov[(i, i)] += (1e-10 - min_eig).max(0.0);
                    }
                }
                cov.cholesky().ok_or(PrvError::NonPsdCovariance)
            },
            Ok,
        )
    }

    fn run_paths(
        &self,
        mean: &State,
        l: &SMatrix<f64, 8, 8>,
        horizon: usize,
        shocks: &[ShockSpec],
        n_paths: usize,
        rng: &mut StdRng,
    ) -> Vec<Vec<State>> {
        let mut all_paths: Vec<Vec<State>> = Vec::with_capacity(n_paths);

        for _ in 0..n_paths {
            let standard_normal: nalgebra::SVector<f64, 8> =
                nalgebra::SVector::from_fn(|_, _| rng.sample(StandardNormal));
            let mut init_vec = *mean.as_vector();
            for i in 0..8 {
                let mut sum = 0.0;
                for j in 0..8 {
                    sum = l[(i, j)].mul_add(standard_normal[j], sum);
                }
                init_vec[i] += sum;
            }
            let initial = State(init_vec);
            let path = self.run_single_path(&initial, horizon, shocks, rng);
            all_paths.push(path);
        }

        all_paths
    }

    #[allow(clippy::unused_self)]
    fn run_single_path(
        &self,
        initial: &State,
        horizon: usize,
        shocks: &[ShockSpec],
        rng: &mut StdRng,
    ) -> Vec<State> {
        let mut path = vec![initial.clone()];
        let mut state = initial.clone();

        for _ in 0..horizon {
            for shock in shocks {
                shock.apply(&mut state, rng);
            }
            path.push(state.clone());
        }

        path
    }

    #[allow(
        clippy::unused_self,
        clippy::needless_pass_by_value,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn compute_results(
        &self,
        all_paths: Vec<Vec<State>>,
        n_paths: usize,
        horizon: usize,
    ) -> Result<MonteCarloResults, PrvError> {
        let sample_count = n_paths * (horizon + 1);
        let final_states: Vec<State> = all_paths.iter().filter_map(|p| p.last().cloned()).collect();

        if final_states.is_empty() {
            return Err(PrvError::NonPsdCovariance);
        }

        let mean_state = self.compute_mean(&final_states);
        let median_state = self.compute_median(&final_states);
        let quantiles = self.compute_quantiles(&final_states);
        let (
            regime_distribution,
            regime_transition,
            capacity_increase,
            capacity_decline,
            investment_increase,
            investment_decline,
        ) = self.compute_regime_stats(&final_states, &mean_state);
        let tail_risk = self.compute_tail_risk(&final_states);

        Ok(MonteCarloResults {
            mean: mean_state,
            median: median_state,
            quantiles,
            probabilities: MonteCarloProbabilities {
                capacity_increase,
                capacity_decline,
                investment_increase,
                investment_decline,
                regime_transition,
            },
            tail_risk,
            regime_distribution,
            seed: self.seed,
            sample_count,
        })
    }

    #[allow(clippy::unused_self, clippy::cast_precision_loss)]
    fn compute_mean(&self, final_states: &[State]) -> State {
        let mut mean_vec = nalgebra::SVector::<f64, 8>::zeros();
        for s in final_states {
            mean_vec += s.as_vector();
        }
        mean_vec /= final_states.len() as f64;
        State(mean_vec)
    }

    #[allow(clippy::unused_self)]
    fn compute_median(&self, final_states: &[State]) -> State {
        let mut sorted: Vec<nalgebra::SVector<f64, 8>> =
            final_states.iter().map(|s| *s.as_vector()).collect();
        sorted.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));
        State(sorted[sorted.len() / 2])
    }

    #[allow(
        clippy::unused_self,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::shadow_reuse
    )]
    fn compute_quantiles(&self, final_states: &[State]) -> Vec<(f64, State)> {
        let mut sorted: Vec<nalgebra::SVector<f64, 8>> =
            final_states.iter().map(|s| *s.as_vector()).collect();
        sorted.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));
        let quantile_levels = [0.01, 0.05, 0.10, 0.50, 0.90, 0.95, 0.99];
        let mut quantiles = Vec::with_capacity(quantile_levels.len());
        for &q in &quantile_levels {
            let q_idx = ((sorted.len() as f64) * q).round() as usize;
            let q_idx = q_idx.min(sorted.len() - 1);
            quantiles.push((q, State(sorted[q_idx])));
        }
        quantiles
    }

    #[allow(clippy::unused_self, clippy::cast_precision_loss)]
    fn compute_regime_stats(
        &self,
        final_states: &[State],
        mean_state: &State,
    ) -> (
        HashMap<prv_core::Regime, usize>,
        HashMap<prv_core::Regime, f64>,
        f64,
        f64,
        f64,
        f64,
    ) {
        let mut regime_distribution: HashMap<prv_core::Regime, usize> = HashMap::new();
        let mut regime_transition: HashMap<prv_core::Regime, f64> = HashMap::new();
        let mut capacity_increase = 0.0;
        let mut capacity_decline = 0.0;
        let mut investment_increase = 0.0;
        let mut investment_decline = 0.0;

        for s in final_states {
            let r = prv_core::Regime::from_pressure(s);
            *regime_distribution.entry(r).or_default() += 1;
            if s.capacity() > mean_state.capacity() {
                capacity_increase += 1.0;
            } else {
                capacity_decline += 1.0;
            }
            if s.investment() > mean_state.investment() {
                investment_increase += 1.0;
            } else {
                investment_decline += 1.0;
            }
        }

        let n = final_states.len() as f64;
        capacity_increase /= n;
        capacity_decline /= n;
        investment_increase /= n;
        investment_decline /= n;

        let total = n.max(1.0);
        for (r, count) in &regime_distribution {
            regime_transition.insert(*r, *count as f64 / total);
        }

        (
            regime_distribution,
            regime_transition,
            capacity_increase,
            capacity_decline,
            investment_increase,
            investment_decline,
        )
    }

    #[allow(
        clippy::unused_self,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn compute_tail_risk(&self, final_states: &[State]) -> f64 {
        let mut tail_values: Vec<f64> = final_states
            .iter()
            .map(|s| s.geopolitical_load() + s.housing_pressure() + s.migration_pressure())
            .collect();
        tail_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let tail_idx = ((tail_values.len() as f64) * 0.95).round() as usize;
        tail_values[tail_idx.min(tail_values.len() - 1)]
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
    use crate::ShockSpec;
    use crate::ShockType;

    #[test]
    fn simulate_returns_expected_structure() {
        let simulator = Simulator::new(42);
        let mean = State::default();
        let covariance = SMatrix::<f64, 8, 8>::identity();
        let shocks = vec![ShockSpec {
            shock_type: ShockType::Demand,
            amplitude: 0.1,
            persistence: 0.5,
            autocorrelation: 0.0,
        }];
        let result = simulator
            .simulate(&mean, &covariance, 10, 5, &shocks)
            .unwrap();
        assert_eq!(result.seed, 42);
        assert_eq!(result.sample_count, 10 * 6);
        assert_eq!(result.quantiles.len(), 7);
    }

    #[test]
    fn simulate_deterministic_with_same_seed() {
        let simulator1 = Simulator::new(123);
        let simulator2 = Simulator::new(123);
        let mean = State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let covariance = SMatrix::<f64, 8, 8>::identity() * 0.1;
        let shocks: Vec<ShockSpec> = vec![];
        let result1 = simulator1
            .simulate(&mean, &covariance, 5, 3, &shocks)
            .unwrap();
        let result2 = simulator2
            .simulate(&mean, &covariance, 5, 3, &shocks)
            .unwrap();
        assert_eq!(result1.mean.as_vector(), result2.mean.as_vector());
    }

    #[test]
    fn seed_reproducibility() {
        let simulator1 = Simulator::new(999);
        let simulator2 = Simulator::new(999);
        let mean = State::default();
        let covariance = SMatrix::<f64, 8, 8>::identity() * 0.5;
        let shocks = vec![ShockSpec {
            shock_type: ShockType::Demand,
            amplitude: 0.1,
            persistence: 0.5,
            autocorrelation: 0.0,
        }];
        let result1 = simulator1
            .simulate(&mean, &covariance, 20, 4, &shocks)
            .unwrap();
        let result2 = simulator2
            .simulate(&mean, &covariance, 20, 4, &shocks)
            .unwrap();
        assert_eq!(result1.mean.as_vector(), result2.mean.as_vector());
        assert_eq!(result1.median.as_vector(), result2.median.as_vector());
        assert_eq!(result1.tail_risk, result2.tail_risk);
    }

    #[test]
    fn distribution_stability_with_large_sample() {
        let simulator = Simulator::new(7);
        let mean = State::default();
        let covariance = SMatrix::<f64, 8, 8>::identity() * 0.2;
        let result = simulator
            .simulate(&mean, &covariance, 1000, 3, &[])
            .unwrap();
        assert!(result.mean.as_vector().iter().all(|&x| x.is_finite()));
        assert!(result.quantiles.len() == 7);
        for (_, state) in &result.quantiles {
            assert!(state.as_vector().iter().all(|&x| x.is_finite()));
        }
    }

    #[test]
    fn sample_convergence_tail_risk_bounded() {
        let simulator = Simulator::new(11);
        let mean = State::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let covariance = SMatrix::<f64, 8, 8>::identity() * 0.1;
        let result = simulator
            .simulate(&mean, &covariance, 500, 10, &[])
            .unwrap();
        assert!(result.tail_risk.is_finite());
    }
}
