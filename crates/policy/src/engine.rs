use crate::{Regime, State};
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
        _mc: &prv_monte_carlo::MonteCarloResults,
        regime: &Regime,
    ) -> PolicyDistribution {
        let _bias = self.regime_response(regime);
        let mut distribution = HashMap::new();

        for instrument in [
            PolicyInstrument::InterestRate,
            PolicyInstrument::QuantitativeEasing,
            PolicyInstrument::SovereignWealthFundDeployment,
            PolicyInstrument::FiscalSpending,
            PolicyInstrument::Taxation,
            PolicyInstrument::InfrastructureInvestment,
            PolicyInstrument::MigrationCapacity,
        ] {
            distribution.insert(instrument, 0.0);
        }

        PolicyDistribution {
            recommended_action_distribution: distribution,
            expected_state_change: None,
            downside_distribution: None,
            tail_risk: 0.0,
            constraint_violations: Vec::new(),
        }
    }

    #[must_use]
    pub const fn regime_response(&self, regime: &Regime) -> PolicyBias {
        match regime {
            Regime::Saturation => PolicyBias::Contraction,
            Regime::Contraction => PolicyBias::Stimulus,
            Regime::Expansion | Regime::Recovery => PolicyBias::Neutral,
            Regime::StructuralShock => PolicyBias::Emergency,
        }
    }
}
