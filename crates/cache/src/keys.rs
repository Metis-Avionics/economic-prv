//! Canonical cache key builders (`namespace:key`).
//!
//! String builders are the stable API; typed helpers return validated
//! keys for the same namespaces.

/// Simulation results key per case.
#[must_use]
pub fn simulation_results_key(case_id: &str) -> String {
    format!("simulation:results:{case_id}")
}

/// EKF state key per timestamp.
#[must_use]
pub fn ekf_state_key(timestamp: &str) -> String {
    format!("ekf:state:{timestamp}")
}

/// Historical data key per source.
#[must_use]
pub fn historical_data_key(source: &str) -> String {
    format!("data:historical:{source}")
}

/// Policy decision key per regime.
#[must_use]
pub fn policy_decision_key(regime: &str) -> String {
    format!("policy:decision:{regime}")
}

/// Namespace of a canonical key (prefix before the first `:`).
#[must_use]
pub fn namespace_of(key: &str) -> &str {
    key.split(':').next().unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_canonical() {
        assert_eq!(
            simulation_results_key("case-a"),
            "simulation:results:case-a"
        );
        assert_eq!(ekf_state_key("2024-01-01"), "ekf:state:2024-01-01");
        assert_eq!(historical_data_key("csv"), "data:historical:csv");
        assert_eq!(
            policy_decision_key("expansion"),
            "policy:decision:expansion"
        );
        assert_eq!(namespace_of("simulation:results:case-a"), "simulation");
    }
}
