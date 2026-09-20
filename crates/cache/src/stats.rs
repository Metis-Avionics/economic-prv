//! Read-heavy per-namespace operation counters.
//!
//! Backed by [`dashmap::DashMap`] so telemetry readers never block
//! writers (and vice versa).

use dashmap::DashMap;
use std::collections::BTreeMap;

/// Operation counters keyed `"<op>:<namespace>"` (e.g. `"set:solver"`).
#[derive(Debug, Default)]
pub struct CacheStats {
    counts: DashMap<String, u64>,
}

impl CacheStats {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one operation against a namespace.
    pub fn record(&self, op: &str, namespace: &str) {
        let counter = format!("{op}:{namespace}");
        self.counts
            .entry(counter)
            .and_modify(|v| *v = v.saturating_add(1))
            .or_insert(1);
    }

    /// Current count for one `"<op>:<namespace>"` counter (0 if absent).
    #[must_use]
    pub fn get(&self, op: &str, namespace: &str) -> u64 {
        let counter = format!("{op}:{namespace}");
        self.counts.get(&counter).map_or(0, |v| *v)
    }

    /// Deterministic snapshot for telemetry export.
    #[must_use]
    pub fn snapshot(&self) -> BTreeMap<String, u64> {
        self.counts
            .iter()
            .map(|e| (e.key().clone(), *e.value()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_snapshots() {
        let s = CacheStats::new();
        s.record("set", "simulation");
        s.record("set", "simulation");
        s.record("get", "ekf");
        assert_eq!(s.get("set", "simulation"), 2);
        assert_eq!(s.get("get", "ekf"), 1);
        assert_eq!(s.get("invalidate", "simulation"), 0);
        let snap = s.snapshot();
        assert_eq!(snap.get("set:simulation"), Some(&2));
    }
}
