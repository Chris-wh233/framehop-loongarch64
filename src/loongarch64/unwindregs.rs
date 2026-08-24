use core::fmt::Debug;

use crate::display_utils::HexNum;

/// The registers used for unwinding on LoongArch64. We only need ra (r1),
/// sp (r3), and fp (r22 / s0).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct UnwindRegsLoongArch64 {
    ra: u64,
    sp: u64,
    fp: u64,
}

impl UnwindRegsLoongArch64 {
    /// Create a set of unwind register values.
    pub fn new(ra: u64, sp: u64, fp: u64) -> Self {
        Self { ra, sp, fp }
    }

    /// Get the stack pointer value (r3).
    #[inline(always)]
    pub fn sp(&self) -> u64 {
        self.sp
    }

    /// Set the stack pointer value (r3).
    #[inline(always)]
    pub fn set_sp(&mut self, sp: u64) {
        self.sp = sp
    }

    /// Get the frame pointer value (r22 / s0).
    #[inline(always)]
    pub fn fp(&self) -> u64 {
        self.fp
    }

    /// Set the frame pointer value (r22 / s0).
    #[inline(always)]
    pub fn set_fp(&mut self, fp: u64) {
        self.fp = fp
    }

    /// Get the return address register value (r1).
    #[inline(always)]
    pub fn ra(&self) -> u64 {
        self.ra
    }

    /// Set the return address register value (r1).
    #[inline(always)]
    pub fn set_ra(&mut self, ra: u64) {
        self.ra = ra
    }
}

impl Debug for UnwindRegsLoongArch64 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UnwindRegsLoongArch64")
            .field("ra", &HexNum(self.ra))
            .field("sp", &HexNum(self.sp))
            .field("fp", &HexNum(self.fp))
            .finish()
    }
}
