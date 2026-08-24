use super::super::unwind_rule::UnwindRuleLoongArch64;

pub fn unwind_rule_from_detected_epilogue(
    _function_bytes: &[u8],
    _pc_offset: usize,
) -> Option<UnwindRuleLoongArch64> {
    None
}
