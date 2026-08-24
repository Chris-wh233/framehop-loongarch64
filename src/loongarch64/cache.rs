use super::unwind_rule::*;
use crate::cache::*;

/// The unwinder cache type for [`UnwinderLoongArch64`](super::UnwinderLoongArch64).
pub struct CacheLoongArch64<P: AllocationPolicy = MayAllocateDuringUnwind>(
    pub Cache<UnwindRuleLoongArch64, P>,
);

impl CacheLoongArch64<MayAllocateDuringUnwind> {
    /// Create a new cache.
    pub fn new() -> Self {
        Self(Cache::new())
    }
}

impl<P: AllocationPolicy> CacheLoongArch64<P> {
    /// Create a new cache.
    pub fn new_in() -> Self {
        Self(Cache::new())
    }

    /// Returns a snapshot of the cache usage statistics.
    pub fn stats(&self) -> CacheStats {
        self.0.rule_cache.stats()
    }
}

impl<P: AllocationPolicy> Default for CacheLoongArch64<P> {
    fn default() -> Self {
        Self::new_in()
    }
}
