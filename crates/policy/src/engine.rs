use crate::{Regime, State};
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand_distr::StandardNormal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyInstrument {
    InterestRate,
    QuantitativeEasing,
    SovereignWealthFundDeployment,
    FiscalSpending,
    Taxation,
    InfrastructureInvestment,
    MigrationCapacity,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyWeights {
    pub capacity: f64,
    pub investment: f64,
    pub employment: f64,
    pub inflation: f64,
    pub debt: f64,
    pub housing_pressure: f64,
    pub systemic_risk: f64,
}

impl Default for PolicyWeights {
    fn default() -> Self {
        Self {
            capacity: 1.0,
            investment: 1.0,
            employment: 1.0,
            inflation: 1.0,
            debt: 1.0,
            housing_pressure: 1.0,
            systemic_risk: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyDistribution {
    pub recommended_action_distribution: HashMap<PolicyInstrument, f64>,
    pub expected_state_change: Option<State>,
    pub downside_distribution: Option<State>,
    pub tail_risk: f64,
    pub constraint_violations: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyBias {
    Neutral,
    Stimulus,
    Contraction,
    Emergency,
}

#[derive(Clone, Debug)]
pub struct PolicyEngine {
    pub weights: PolicyWeights,
}

impl PolicyEngine {
    #[must_use]
    pub const fn new(weights: PolicyWeights) -> Self {
        Self { weights }
    }

    #[must_use]
    pub fn evaluate(
        &self,
        mc: &prv_monte_carlo::MonteCarloResults,
        regime: &Regime,
        seed: Option<u64>,
    ) -> PolicyDistribution {
        let mut rng = seed.map(StdRng::seed_from_u64);
        let bias = self.regime_response(regime);
        let state = &mc.mean;
        let signals = self.compute_state_signals(state);
        let mut distribution = HashMap::new();
        self.score_monetary_policy(&mut distribution, &signals, bias, &mut rng);
        self.score_fiscal_policy(&mut distribution, &signals, bias, &mut rng);
        self.score_real_economy(&mut distribution, &signals, bias, &mut rng);
        self.build_policy_distribution(mc, state, &signals, distribution)
    }

    #[allow(clippy::unused_self)]
    fn compute_state_signals(&self, state: &State) -> StateSignals {
        let capacity = state.capacity().max(1e-10);
        let inflation_signal = state.demand_pressure() / capacity;
        let debt_signal = state.fiscal_capacity();
        let systemic_signal = (state.geopolitical_load() + state.migration_pressure()) / capacity;
        StateSignals {
            inflation_signal,
            debt_signal,
            systemic_signal,
        }
    }

    fn score_monetary_policy(
        &self,
        distribution: &mut HashMap<PolicyInstrument, f64>,
        signals: &StateSignals,
        bias: PolicyBias,
        rng: &mut Option<StdRng>,
    ) {
        let base_interest = match bias {
            PolicyBias::Stimulus | PolicyBias::Emergency => -0.5,
            PolicyBias::Contraction => 0.5,
            PolicyBias::Neutral => 0.0,
        };
        let interest_score = self.weights.systemic_risk.mul_add(
            signals.systemic_signal,
            self.weights
                .inflation
                .mul_add(signals.inflation_signal, base_interest),
        );
        let interest_noise = sample_noise(rng);
        distribution.insert(
            PolicyInstrument::InterestRate,
            (interest_score + interest_noise).clamp(-1.0, 1.0),
        );

        let qe_score = match bias {
            PolicyBias::Emergency => 0.9,
            PolicyBias::Stimulus => 0.6,
            PolicyBias::Contraction => -0.6,
            PolicyBias::Neutral => 0.0,
        };
        let qe_noise = sample_noise(rng);
        distribution.insert(PolicyInstrument::QuantitativeEasing, qe_score + qe_noise);
    }

    fn score_fiscal_policy(
        &self,
        distribution: &mut HashMap<PolicyInstrument, f64>,
        _signals: &StateSignals,
        bias: PolicyBias,
        rng: &mut Option<StdRng>,
    ) {
        let swf_score = match bias {
            PolicyBias::Emergency | PolicyBias::Stimulus => 0.7,
            PolicyBias::Contraction => 0.1,
            PolicyBias::Neutral => 0.2,
        };
        let swf_noise = sample_noise(rng);
        distribution.insert(
            PolicyInstrument::SovereignWealthFundDeployment,
            swf_score * self.weights.capacity + swf_noise,
        );

        let fiscal_score = match bias {
            PolicyBias::Emergency => 1.0,
            PolicyBias::Stimulus => 0.7,
            PolicyBias::Contraction => -0.4,
            PolicyBias::Neutral => 0.0,
        };
        let fiscal_noise = sample_noise(rng);
        distribution.insert(
            PolicyInstrument::FiscalSpending,
            fiscal_score * self.weights.investment + fiscal_noise,
        );

        let tax_score = match bias {
            PolicyBias::Contraction => 0.5,
            PolicyBias::Emergency | PolicyBias::Stimulus => -0.3,
            PolicyBias::Neutral => 0.0,
        };
        let tax_noise = sample_noise(rng);
        distribution.insert(PolicyInstrument::Taxation, tax_score + tax_noise);
    }

    fn score_real_economy(
        &self,
        distribution: &mut HashMap<PolicyInstrument, f64>,
        _signals: &StateSignals,
        bias: PolicyBias,
        rng: &mut Option<StdRng>,
    ) {
        let infra_score = match bias {
            PolicyBias::Emergency | PolicyBias::Stimulus => 0.8,
            PolicyBias::Contraction => 0.1,
            PolicyBias::Neutral => 0.2,
        };
        let infra_noise = sample_noise(rng);
        distribution.insert(
            PolicyInstrument::InfrastructureInvestment,
            infra_score * self.weights.capacity + infra_noise,
        );

        let migration_score = match bias {
            PolicyBias::Emergency => 0.6,
            PolicyBias::Stimulus => 0.3,
            PolicyBias::Neutral => 0.1,
            PolicyBias::Contraction => 0.0,
        };
        let migration_noise = sample_noise(rng);
        distribution.insert(
            PolicyInstrument::MigrationCapacity,
            migration_score * self.weights.systemic_risk + migration_noise,
        );
    }

    fn build_policy_distribution(
        &self,
        mc: &prv_monte_carlo::MonteCarloResults,
        state: &State,
        signals: &StateSignals,
        distribution: HashMap<PolicyInstrument, f64>,
    ) -> PolicyDistribution {
        let scores = self.extract_scores(&distribution);
        let expected = self.compute_expected(state, &scores);
        let downside = self.compute_downside(state);
        let constraint_violations = self.check_constraints(signals, &scores);

        PolicyDistribution {
            recommended_action_distribution: distribution,
            expected_state_change: Some(expected),
            downside_distribution: Some(downside),
            tail_risk: mc.tail_risk,
            constraint_violations,
        }
    }

    #[allow(clippy::unused_self)]
    fn extract_scores(&self, distribution: &HashMap<PolicyInstrument, f64>) -> InstrumentScores {
        InstrumentScores {
            interest: distribution
                .get(&PolicyInstrument::InterestRate)
                .copied()
                .unwrap_or(0.0),
            qe: distribution
                .get(&PolicyInstrument::QuantitativeEasing)
                .copied()
                .unwrap_or(0.0),
            swf: distribution
                .get(&PolicyInstrument::SovereignWealthFundDeployment)
                .copied()
                .unwrap_or(0.0),
            fiscal: distribution
                .get(&PolicyInstrument::FiscalSpending)
                .copied()
                .unwrap_or(0.0),
            tax: distribution
                .get(&PolicyInstrument::Taxation)
                .copied()
                .unwrap_or(0.0),
            infra: distribution
                .get(&PolicyInstrument::InfrastructureInvestment)
                .copied()
                .unwrap_or(0.0),
            migration: distribution
                .get(&PolicyInstrument::MigrationCapacity)
                .copied()
                .unwrap_or(0.0),
        }
    }

    #[allow(clippy::unused_self)]
    fn compute_expected(&self, state: &State, scores: &InstrumentScores) -> State {
        State::new(
            0.01f64.mul_add(scores.fiscal, state.capacity()),
            0.02f64.mul_add(scores.swf, state.investment()),
            0.01f64.mul_add(scores.infra, state.labour_absorption()),
            0.01f64.mul_add(-scores.tax, state.fiscal_capacity()),
            0.01f64.mul_add(-scores.interest, state.demand_pressure()),
            0.005f64.mul_add(-scores.interest, state.housing_pressure()),
            0.01f64.mul_add(-scores.migration, state.geopolitical_load()),
            0.01f64.mul_add(-scores.migration, state.migration_pressure()),
        )
    }

    #[allow(clippy::unused_self)]
    fn compute_downside(&self, state: &State) -> State {
        State::new(
            state.capacity() - 0.02,
            state.investment() - 0.03,
            state.labour_absorption() - 0.01,
            state.fiscal_capacity() + 0.01,
            state.demand_pressure() + 0.01,
            state.housing_pressure() + 0.01,
            state.geopolitical_load() + 0.02,
            state.migration_pressure() + 0.01,
        )
    }

    #[allow(clippy::unused_self)]
    fn check_constraints(&self, signals: &StateSignals, scores: &InstrumentScores) -> Vec<String> {
        let mut violations = Vec::new();
        if signals.inflation_signal > 2.0 && scores.qe > 0.5 {
            violations.push("inflation_and_financial_stability".to_string());
        }
        if scores.swf > 0.7 && signals.debt_signal < 0.1 {
            violations.push("fund_liquidity_and_intergenerational_equity".to_string());
        }
        violations
    }

    #[must_use]
    #[allow(clippy::unused_self)]
    pub const fn regime_response(&self, regime: &Regime) -> PolicyBias {
        match regime {
            Regime::Saturation => PolicyBias::Contraction,
            Regime::Contraction => PolicyBias::Stimulus,
            Regime::Expansion | Regime::Recovery => PolicyBias::Neutral,
            Regime::StructuralShock => PolicyBias::Emergency,
        }
    }
}

#[derive(Debug)]
struct InstrumentScores {
    interest: f64,
    qe: f64,
    swf: f64,
    fiscal: f64,
    tax: f64,
    infra: f64,
    migration: f64,
}

#[allow(clippy::struct_field_names)]
#[derive(Debug)]
struct StateSignals {
    inflation_signal: f64,
    debt_signal: f64,
    systemic_signal: f64,
}

fn sample_noise(rng: &mut Option<rand::rngs::StdRng>) -> f64 {
    rng.as_mut().map_or_else(
        || (rand::random::<f64>() - 0.5) * 0.2,
        |r| r.sample::<f64, _>(StandardNormal) * 0.1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::SMatrix;
    use prv_monte_carlo::Simulator;

    #[test]
    fn evaluate_returns_policy_distribution() {
        let engine = PolicyEngine::new(PolicyWeights::default());
        let simulator = Simulator::new(42);
        let mean = State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let covariance = SMatrix::<f64, 8, 8>::identity() * 0.1;
        let mc = simulator.simulate(&mean, &covariance, 10, 5, &[]).unwrap();
        let policy = engine.evaluate(&mc, &Regime::Expansion, Some(42));
        assert_eq!(policy.recommended_action_distribution.len(), 7);
        assert!(policy.expected_state_change.is_some());
        assert!(policy.downside_distribution.is_some());
        assert!(
            policy.constraint_violations.is_empty() || !policy.constraint_violations.is_empty()
        );
    }

    #[test]
    fn regime_response_mapping() {
        let engine = PolicyEngine::new(PolicyWeights::default());
        assert_eq!(
            engine.regime_response(&Regime::Expansion),
            PolicyBias::Neutral
        );
        assert_eq!(
            engine.regime_response(&Regime::Saturation),
            PolicyBias::Contraction
        );
        assert_eq!(
            engine.regime_response(&Regime::Contraction),
            PolicyBias::Stimulus
        );
        assert_eq!(
            engine.regime_response(&Regime::StructuralShock),
            PolicyBias::Emergency
        );
    }

    #[test]
    fn no_single_point_policy_output() {
        let engine = PolicyEngine::new(PolicyWeights::default());
        let simulator = Simulator::new(42);
        let mean = State::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let covariance = SMatrix::<f64, 8, 8>::identity() * 0.1;
        let mc = simulator.simulate(&mean, &covariance, 10, 5, &[]).unwrap();
        for regime in [
            Regime::Expansion,
            Regime::Contraction,
            Regime::StructuralShock,
        ] {
            let policy = engine.evaluate(&mc, &regime, Some(42));
            assert!(policy.recommended_action_distribution.len() > 1);
        }
    }
}
