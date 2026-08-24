use super::unwind_rule::UnwindRuleLoongArch64;
use super::unwindregs::UnwindRegsLoongArch64;
use crate::arch::Arch;

/// The LoongArch64 CPU architecture.
pub struct ArchLoongArch64;
impl Arch for ArchLoongArch64 {
    type UnwindRule = UnwindRuleLoongArch64;
    type UnwindRegs = UnwindRegsLoongArch64;
}
