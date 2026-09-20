//! [`NsCache`]: policy-routed cache façade over `thesix`.
//!
//! Tier selection, single-flight population and generation tracking all
//! live inside `thesix` (`CacheManager` + policy + `Cachelito`). This
//! layer only adds canonical PRV keys, [`CacheStats`] counters and
//! [`KeyIndex`] bookkeeping — all read-heavy state in [`dashmap`].

use crate::index::KeyIndex;
use crate::keys::namespace_of;
use crate::stats::CacheStats;
use std::sync::Arc;
use thesix::{
    CacheContext, CacheError, CacheManager, CacheTier, Cachelito, DefaultPolicy, IdentityContext,
    L0Stub, L1Stub, L2Stub, L3Stub, L4Stub, L5Stub, MemoryPool, TierRegistry,
};

/// Cache façade. Cloneable handle to shared manager + stats + index.
#[derive(Clone)]
pub struct NsCache {
    manager: Arc<CacheManager<String, String, DefaultPolicy>>,
    stats: Arc<CacheStats>,
    index: Arc<KeyIndex>,
}

impl Default for NsCache {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for NsCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NsCache")
            .field("stats", &self.stats.snapshot())
            .field("indexed_keys", &self.index.len())
            .finish_non_exhaustive()
    }
}

impl NsCache {
    /// Build with six in-memory stub tiers (no external services).
    ///
    /// # Panics
    ///
    /// Panics if the `MemoryPool` allocation fails (unlikely with the default size).
    #[must_use]
    pub fn new() -> Self {
        let tiers: Vec<Arc<dyn CacheTier<String>>> = vec![
            Arc::new(L0Stub::new()),
            Arc::new(L1Stub::new()),
            Arc::new(L2Stub::new()),
            Arc::new(L3Stub::new()),
            Arc::new(L4Stub::new()),
            Arc::new(L5Stub::new()),
        ];
        Self::with_tiers(tiers)
    }

    /// Build over caller-supplied tiers (e.g. `TestTier`s, Redis/Sled
    /// backends). The tier vector order is L0..L5.
    ///
    /// # Panics
    ///
    /// Panics if the `MemoryPool` allocation fails (unlikely with the default size).
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn with_tiers(tiers: Vec<Arc<dyn CacheTier<String>>>) -> Self {
        let manager = Arc::new(CacheManager::new(
            DefaultPolicy,
            Cachelito::new(),
            TierRegistry::new(),
            tiers,
            MemoryPool::new(1024).expect("pool allocation failed"),
        ));
        Self {
            manager,
            stats: Arc::new(CacheStats::new()),
            index: Arc::new(KeyIndex::new()),
        }
    }

    /// Authenticated request context for one call site.
    #[must_use]
    pub fn ctx(user: &str, roles: Vec<String>, tenant: &str) -> CacheContext {
        CacheContext::new(IdentityContext::new(
            user.to_string(),
            roles,
            tenant.to_string(),
        ))
    }

    /// Anonymous request context (writes are policy-gated).
    #[must_use]
    pub fn anonymous_ctx() -> CacheContext {
        CacheContext::anonymous()
    }

    /// Tier lookup per policy, with lazy TTL expiry.
    ///
    /// # Errors
    ///
    /// Returns `CacheError` if the underlying cache lookup fails.
    pub async fn get(&self, key: &str, ctx: &CacheContext) -> Result<Option<String>, CacheError> {
        self.stats.record("get", namespace_of(key));
        self.manager.get(&key.to_string(), ctx).await
    }

    /// `get`, else single-flight populate (exactly one fetch wins under
    /// concurrent misses).
    ///
    /// # Errors
    ///
    /// Returns `CacheError` if the underlying cache or fetch fails.
    #[allow(clippy::future_not_send)]
    pub async fn get_or_fetch<F, Fut>(
        &self,
        key: &str,
        ctx: &CacheContext,
        fetch: F,
    ) -> Result<String, CacheError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<String, CacheError>>,
    {
        self.stats.record("get_or_fetch", namespace_of(key));
        let value = self
            .manager
            .get_or_fetch(&key.to_string(), ctx, fetch)
            .await?;
        self.index.insert(key);
        Ok(value)
    }

    /// Policy-routed write (authz-gated by the policy engine).
    ///
    /// # Errors
    ///
    /// Returns `CacheError` if the underlying cache rejects the write.
    pub async fn set(
        &self,
        key: &str,
        value: String,
        ctx: &CacheContext,
    ) -> Result<(), CacheError> {
        self.stats.record("set", namespace_of(key));
        self.manager.set(&key.to_string(), value, ctx).await?;
        self.index.insert(key);
        Ok(())
    }

    /// Generation bump — stale in-flight publications are rejected.
    ///
    /// # Errors
    ///
    /// Returns `CacheError` if invalidation fails.
    pub async fn invalidate(&self, key: &str, ctx: &CacheContext) -> Result<(), CacheError> {
        self.stats.record("invalidate", namespace_of(key));
        self.manager.invalidate(&key.to_string(), ctx).await?;
        self.index.remove(key);
        Ok(())
    }

    /// Generation bump + tier eviction.
    ///
    /// # Errors
    ///
    /// Returns `CacheError` if removal fails.
    pub async fn remove(&self, key: &str, ctx: &CacheContext) -> Result<(), CacheError> {
        self.stats.record("remove", namespace_of(key));
        self.manager.remove(&key.to_string(), ctx).await?;
        self.index.remove(key);
        Ok(())
    }

    /// Presence check without fetching.
    ///
    /// # Errors
    ///
    /// Returns `CacheError` if the existence check fails.
    pub async fn exists(&self, key: &str, ctx: &CacheContext) -> Result<bool, CacheError> {
        self.stats.record("exists", namespace_of(key));
        self.manager.exists(&key.to_string(), ctx).await
    }

    /// Stale-while-revalidate refresh. Returns the revalidated value,
    /// or `None` when there is nothing fresh to serve.
    ///
    /// # Errors
    ///
    /// Returns `CacheError` if the refresh or fetch fails.
    #[allow(clippy::future_not_send)]
    pub async fn refresh<F, Fut>(
        &self,
        key: &str,
        ctx: &CacheContext,
        fetch: F,
    ) -> Result<Option<String>, CacheError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<String, CacheError>>,
    {
        self.stats.record("refresh", namespace_of(key));
        let value = self.manager.refresh(&key.to_string(), ctx, fetch).await?;
        if value.is_some() {
            self.index.insert(key);
        }
        Ok(value)
    }

    /// Batch warm: independent `set`s run concurrently via `JoinSet`.
    /// Order-independent; fails fast on the first error (completed tasks
    /// are kept — callers needing atomicity must use a transaction).
    ///
    /// # Errors
    ///
    /// Returns `CacheError::Cancelled` if a spawned task panics.
    pub async fn warm_many(
        &self,
        pairs: &[(String, String)],
        ctx: &CacheContext,
    ) -> Result<(), CacheError> {
        let mut set = tokio::task::JoinSet::new();
        for (k, v) in pairs {
            let this = self.clone();
            let ctx_clone = ctx.clone();
            let key = k.clone();
            let val = v.clone();
            set.spawn(async move { this.set(&key, val, &ctx_clone).await });
        }
        while let Some(r) = set.join_next().await {
            r.map_err(|_| CacheError::Cancelled)??;
        }
        Ok(())
    }

    /// Batch invalidate by keys (maintenance worker path).
    ///
    /// # Errors
    ///
    /// Returns `CacheError::Cancelled` if a spawned task panics.
    pub async fn invalidate_many(
        &self,
        keys: &[String],
        ctx: &CacheContext,
    ) -> Result<(), CacheError> {
        let mut set = tokio::task::JoinSet::new();
        for k in keys {
            let this = self.clone();
            let ctx_clone = ctx.clone();
            let key = k.clone();
            set.spawn(async move { this.invalidate(&key, &ctx_clone).await });
        }
        while let Some(r) = set.join_next().await {
            r.map_err(|_| CacheError::Cancelled)??;
        }
        Ok(())
    }

    /// Deterministic snapshot for telemetry export.
    #[must_use]
    pub fn stats_snapshot(&self) -> std::collections::BTreeMap<String, u64> {
        self.stats.snapshot()
    }

    /// All indexed keys belonging to a subject (namespace).
    #[must_use]
    pub fn keys_for_subject(&self, subject: &str) -> Vec<String> {
        self.index.keys_for_subject(subject)
    }
}
