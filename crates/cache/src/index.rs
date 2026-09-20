//! Read-heavy key → subject index for pattern-scoped lookups.
//!
//! Backed by [`dashmap::DashMap`]: concurrent readers never block cache writers.

use crate::keys::namespace_of;
use dashmap::DashMap;

/// Maps canonical key → subject (here: the key's namespace).
#[derive(Debug, Default)]
pub struct KeyIndex {
    entries: DashMap<String, String>,
}

impl KeyIndex {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Index a key under its namespace-derived subject.
    pub fn insert(&self, key: &str) {
        self.entries
            .insert(key.to_string(), namespace_of(key).to_string());
    }

    /// Subject for a key, if indexed.
    #[must_use]
    pub fn subject_for(&self, key: &str) -> Option<String> {
        self.entries.get(key).map(|v| v.clone())
    }

    /// All indexed keys belonging to a subject.
    #[must_use]
    pub fn keys_for_subject(&self, subject: &str) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.value() == subject)
            .map(|e| e.key().clone())
            .collect()
    }

    /// Drop a key from the index.
    pub fn remove(&self, key: &str) {
        self.entries.remove(key);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_round_trip() {
        let idx = KeyIndex::new();
        assert!(idx.is_empty());
        idx.insert("simulation:results:case-a:7");
        idx.insert("ekf:state:2024-01-01");
        assert_eq!(idx.len(), 2);
        assert_eq!(
            idx.subject_for("simulation:results:case-a:7"),
            Some("simulation".to_string())
        );
        let mut keys = idx.keys_for_subject("simulation");
        keys.sort();
        assert_eq!(keys, vec!["simulation:results:case-a:7".to_string()]);
        idx.remove("simulation:results:case-a:7");
        assert!(idx.subject_for("simulation:results:case-a:7").is_none());
    }
}
