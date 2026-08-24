use super::super::unwind_rule::UnwindRuleLoongArch64;

pub fn unwind_rule_from_detected_prologue(
    _slice_from_start: &[u8],
    _slice_to_end: &[u8],
) -> Option<UnwindRuleLoongArch64> {
    None
}
