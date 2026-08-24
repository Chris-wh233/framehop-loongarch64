use super::arch::ArchLoongArch64;
use crate::macho::{CompactUnwindInfoUnwinderError, CompactUnwindInfoUnwinding, CuiUnwindResult};

impl CompactUnwindInfoUnwinding for ArchLoongArch64 {
    fn unwind_frame(
        _function: macho_unwind_info::Function,
        _is_first_frame: bool,
        _address_offset_within_function: usize,
        _function_bytes: Option<&[u8]>,
    ) -> Result<CuiUnwindResult<Self::UnwindRule>, CompactUnwindInfoUnwinderError> {
        Err(CompactUnwindInfoUnwinderError::Loongarch64Unsupported)
    }

    fn rule_for_stub_helper(
        _offset: u32,
    ) -> Result<CuiUnwindResult<Self::UnwindRule>, CompactUnwindInfoUnwinderError> {
        Err(CompactUnwindInfoUnwinderError::Loongarch64Unsupported)
    }
}
