use core::ops::Deref;

use crate::{
    unwinder::UnwinderInternal, AllocationPolicy, Error, FrameAddress, MayAllocateDuringUnwind,
    Module, Unwinder,
};

use super::{ArchLoongArch64, CacheLoongArch64, UnwindRegsLoongArch64};

/// The unwinder for the LoongArch64 CPU architecture. Use the [`Unwinder`] trait for unwinding.
///
/// Type arguments:
///
///  - `D`: The type for unwind section data in the modules. See [`Module`].
/// -  `P`: The [`AllocationPolicy`].
pub struct UnwinderLoongArch64<D, P = MayAllocateDuringUnwind>(
    UnwinderInternal<D, ArchLoongArch64, P>,
);

impl<D, P> Default for UnwinderLoongArch64<D, P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D, P> Clone for UnwinderLoongArch64<D, P> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<D, P> UnwinderLoongArch64<D, P> {
    /// Create an unwinder for a process.
    pub fn new() -> Self {
        Self(UnwinderInternal::new())
    }
}

impl<D: Deref<Target = [u8]>, P: AllocationPolicy> Unwinder for UnwinderLoongArch64<D, P> {
    type UnwindRegs = UnwindRegsLoongArch64;
    type Cache = CacheLoongArch64<P>;
    type Module = Module<D>;

    fn add_module(&mut self, module: Module<D>) {
        self.0.add_module(module);
    }

    fn remove_module(&mut self, module_address_range_start: u64) {
        self.0.remove_module(module_address_range_start);
    }

    fn max_known_code_address(&self) -> u64 {
        self.0.max_known_code_address()
    }

    fn unwind_frame<F>(
        &self,
        address: FrameAddress,
        regs: &mut UnwindRegsLoongArch64,
        cache: &mut CacheLoongArch64<P>,
        read_stack: &mut F,
    ) -> Result<Option<u64>, Error>
    where
        F: FnMut(u64) -> Result<u64, ()>,
    {
        self.0.unwind_frame(address, regs, &mut cache.0, read_stack)
    }
}
