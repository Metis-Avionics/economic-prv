use crate::{ShockSpec, State};
use nalgebra::Cholesky;
use rand::prelude::*;
use rand_distr::StandardNormal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    #[must_use]
    /// Samples states from a multivariate normal distribution.
    ///
    /// # Panics
    ///
    /// Panics if Cholesky decomposition fails even after eigenvalue clipping.
    pub fn sample_state(
        &self,
        mean: &State,
        covariance: &nalgebra::SMatrix<f64, 8, 8>,
        n: usize,
    ) -> Vec<State> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed);
        let mut samples = Vec::with_capacity(n);

        let chol = Cholesky::new(*covariance).unwrap_or_else(|| {
            let mut cov = *covariance;
            let eigen = cov.symmetric_eigenvalues();
            let min_eig = eigen.min();
            if min_eig < 1e-10 {
                for i in 0..8 {
                    cov[(i, i)] += (1e-10 - min_eig).max(0.0);
                }
            }
            Cholesky::new(cov).expect("Cholesky decomposition after eigenvalue clipping")
        });

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

        samples
    }

    #[must_use]
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

    #[must_use]
    pub fn simulate(
        &self,
        _mean: &State,
        _covariance: &nalgebra::SMatrix<f64, 8, 8>,
        n_paths: usize,
        horizon: usize,
    ) -> MonteCarloResults {
        MonteCarloResults {
            mean: State::default(),
            median: State::default(),
            quantiles: Vec::new(),
            probabilities: MonteCarloProbabilities {
                capacity_increase: 0.0,
                capacity_decline: 0.0,
                investment_increase: 0.0,
                investment_decline: 0.0,
                regime_transition: HashMap::new(),
            },
            tail_risk: 0.0,
            regime_distribution: HashMap::new(),
            seed: self.seed,
            sample_count: n_paths * horizon,
        }
    }
}
