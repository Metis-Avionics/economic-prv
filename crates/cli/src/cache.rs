use std::sync::Arc;
use thesix::{CacheTier, KeyRef, L0Stub};

#[derive(Clone, Debug)]
pub struct CliCache {
    tier: Arc<L0Stub<String>>,
}

impl Default for CliCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CliCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tier: Arc::new(L0Stub::new()),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let key_ref = KeyRef::from(key.as_bytes());
        self.tier.get(&key_ref).ok().flatten()
    }

    pub fn set(&self, key: &str, value: String) {
        let key_ref = KeyRef::from(key.as_bytes());
        let _ = self.tier.set(&key_ref, value, None);
    }
}
