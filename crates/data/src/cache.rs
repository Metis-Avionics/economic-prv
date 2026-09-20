use crate::DataFrame;
use std::sync::Arc;
use thesix::{CacheTier, KeyRef, L0Stub};

#[derive(Clone, Debug)]
pub struct DataCache {
    tier: Arc<L0Stub<DataFrame>>,
}

impl Default for DataCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DataCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tier: Arc::new(L0Stub::new()),
        }
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<DataFrame> {
        let key_ref = KeyRef::from(key.as_bytes());
        self.tier.get(&key_ref).ok().flatten()
    }

    #[allow(clippy::let_underscore_must_use)]
    pub fn set(&self, key: &str, value: DataFrame) {
        let key_ref = KeyRef::from(key.as_bytes());
        let _ = self.tier.set(&key_ref, value, None);
    }

    #[allow(clippy::let_underscore_must_use)]
    pub fn clear(&self) {
        let empty: &[u8] = &[];
        let _ = self.tier.remove(&KeyRef::from(empty));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_set_then_get_returns_value() {
        let cache = DataCache::new();
        let df = DataFrame::new(vec!["a".to_string()], vec![vec![1.0, 2.0]]);
        cache.set("test-key", df.clone());
        let retrieved = cache.get("test-key").unwrap();
        assert_eq!(retrieved.columns, df.columns);
        assert_eq!(retrieved.data, df.data);
    }

    #[test]
    fn cache_get_missing_returns_none() {
        let cache = DataCache::new();
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn cache_overwrite_returns_latest() {
        let cache = DataCache::new();
        let df1 = DataFrame::new(vec!["a".to_string()], vec![vec![1.0]]);
        let df2 = DataFrame::new(vec!["b".to_string()], vec![vec![2.0]]);
        cache.set("key", df1);
        cache.set("key", df2);
        let retrieved = cache.get("key").unwrap();
        assert_eq!(retrieved.columns, vec!["b".to_string()]);
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn cache_clone_is_independent() {
        let cache = DataCache::new();
        let df = DataFrame::new(vec!["a".to_string()], vec![vec![1.0]]);
        cache.set("key", df);
        let cache2 = cache.clone();
        assert!(cache2.get("key").is_some());
    }
}
