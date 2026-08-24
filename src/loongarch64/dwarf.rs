use gimli::{
    CfaRule, Encoding, EvaluationStorage, LoongArch, Reader, ReaderOffset, Register, RegisterRule,
    UnwindContextStorage, UnwindSection, UnwindTableRow,
};

use super::{
    arch::ArchLoongArch64, unwind_rule::UnwindRuleLoongArch64, unwindregs::UnwindRegsLoongArch64,
};

use crate::unwind_result::UnwindResult;

use crate::dwarf::{
    eval_cfa_rule, eval_register_rule, ConversionError, DwarfUnwindRegs, DwarfUnwinderError,
    DwarfUnwinding,
};

impl DwarfUnwindRegs for UnwindRegsLoongArch64 {
    fn get(&self, register: Register) -> Option<u64> {
        match register {
            LoongArch::R3 => Some(self.sp()),
            LoongArch::R22 => Some(self.fp()),
            LoongArch::R1 => Some(self.ra()),
            _ => None,
        }
    }
}

impl DwarfUnwinding for ArchLoongArch64 {
    fn unwind_frame<F, R, UCS, ES>(
        section: &impl UnwindSection<R>,
        unwind_info: &UnwindTableRow<R::Offset, UCS>,
        encoding: Encoding,
        regs: &mut Self::UnwindRegs,
        is_first_frame: bool,
        read_stack: &mut F,
    ) -> Result<UnwindResult<Self::UnwindRule>, DwarfUnwinderError>
    where
        F: FnMut(u64) -> Result<u64, ()>,
        R: Reader,
        UCS: UnwindContextStorage<R::Offset>,
        ES: EvaluationStorage<R>,
    {
        let cfa_rule = unwind_info.cfa();
        let fp_rule = unwind_info.register(LoongArch::R22);
        let ra_rule = unwind_info.register(LoongArch::R1);

        match translate_into_unwind_rule(cfa_rule, fp_rule.as_ref(), ra_rule.as_ref()) {
            Ok(unwind_rule) => return Ok(UnwindResult::ExecRule(unwind_rule)),
            Err(_err) => {
                // Could not translate into a cacheable unwind rule. Fall back to the generic path.
                // eprintln!("Unwind rule translation failed: {:?}", err);
            }
        }

        let cfa = eval_cfa_rule::<R, _, ES>(section, cfa_rule, encoding, regs)
            .ok_or(DwarfUnwinderError::CouldNotRecoverCfa)?;

        let ra = regs.ra();
        let fp = regs.fp();
        let sp = regs.sp();

        let (fp, ra) = if !is_first_frame {
            if cfa <= sp {
                return Err(DwarfUnwinderError::StackPointerMovedBackwards);
            }
            let fp = eval_register_rule::<R, F, _, ES>(
                section, fp_rule, cfa, encoding, fp, regs, read_stack,
            )
            .ok_or(DwarfUnwinderError::CouldNotRecoverFramePointer)?;
            let ra = eval_register_rule::<R, F, _, ES>(
                section, ra_rule, cfa, encoding, ra, regs, read_stack,
            )
            .ok_or(DwarfUnwinderError::CouldNotRecoverReturnAddress)?;
            (fp, ra)
        } else {
            // For the first frame, be more lenient when encountering errors.
            // TODO: Find evidence of what this gives us. I think on macOS the prologue often has Unknown register rules
            // and we only encounter prologues for the first frame.
            let fp = eval_register_rule::<R, F, _, ES>(
                section, fp_rule, cfa, encoding, fp, regs, read_stack,
            )
            .unwrap_or(fp);
            let ra = eval_register_rule::<R, F, _, ES>(
                section, ra_rule, cfa, encoding, ra, regs, read_stack,
            )
            .unwrap_or(ra);
            (fp, ra)
        };

        regs.set_fp(fp);
        regs.set_sp(cfa);
        regs.set_ra(ra);

        Ok(UnwindResult::Uncacheable(ra))
    }

    fn rule_if_uncovered_by_fde() -> Self::UnwindRule {
        UnwindRuleLoongArch64::NoOpIfFirstFrameOtherwiseFp
    }
}

fn register_rule_to_cfa_offset<RO: ReaderOffset>(
    rule: Option<&RegisterRule<RO>>,
) -> Result<Option<i64>, ConversionError> {
    let Some(rule) = rule else { return Ok(None) };
    match *rule {
        RegisterRule::Undefined | RegisterRule::SameValue => Ok(None),
        RegisterRule::Offset(offset) => Ok(Some(offset)),
        _ => Err(ConversionError::RegisterNotStoredRelativeToCfa),
    }
}

fn translate_into_unwind_rule<RO: ReaderOffset>(
    cfa_rule: &CfaRule<RO>,
    fp_rule: Option<&RegisterRule<RO>>,
    ra_rule: Option<&RegisterRule<RO>>,
) -> Result<UnwindRuleLoongArch64, ConversionError> {
    match cfa_rule {
        CfaRule::RegisterAndOffset { register, offset } => match *register {
            LoongArch::R3 => {
                let sp_offset_by_16 =
                    u16::try_from(offset / 16).map_err(|_| ConversionError::SpOffsetDoesNotFit)?;
                let ra_cfa_offset = register_rule_to_cfa_offset(ra_rule)?;
                let fp_cfa_offset = register_rule_to_cfa_offset(fp_rule)?;
                match (ra_cfa_offset, fp_cfa_offset) {
                    (None, Some(_)) => Err(ConversionError::RestoringFpButNotRa),
                    (None, None) => {
                        match ra_rule {
                            None => {
                                // The column for the return address register was omitted from the DWARF CFI table.
                                // Per spec (at least as of DWARF >= 3), this means that it should be treated
                                // as undefined. However, in practice, it seems that compilers often omit the rule
                                // to say "same value", see https://github.com/gimli-rs/gimli/issues/857 .
                                Ok(UnwindRuleLoongArch64::OffsetSp { sp_offset_by_16 })
                            }
                            Some(RegisterRule::Undefined) => {
                                // The column for the return address was manually set to "undefined"
                                // using DW_CFA_undefined. This usually means that the function never returns
                                // and can be treated as the root of the stack.
                                Ok(
                                    UnwindRuleLoongArch64::OffsetSpIfFirstFrameOtherwiseStackEndsHere {
                                        sp_offset_by_16,
                                    },
                                )
                            }
                            _ => Ok(UnwindRuleLoongArch64::OffsetSp { sp_offset_by_16 }),
                        }
                    }
                    (Some(ra_cfa_offset), None) => {
                        let ra_storage_offset_from_sp_by_8 =
                            i16::try_from((offset + ra_cfa_offset) / 8)
                                .map_err(|_| ConversionError::RaStorageOffsetDoesNotFit)?;
                        Ok(UnwindRuleLoongArch64::OffsetSpAndRestoreRa {
                            sp_offset_by_16,
                            ra_storage_offset_from_sp_by_8,
                        })
                    }
                    (Some(ra_cfa_offset), Some(fp_cfa_offset)) => {
                        let ra_storage_offset_from_sp_by_8 =
                            i16::try_from((offset + ra_cfa_offset) / 8)
                                .map_err(|_| ConversionError::RaStorageOffsetDoesNotFit)?;
                        let fp_storage_offset_from_sp_by_8 =
                            i16::try_from((offset + fp_cfa_offset) / 8)
                                .map_err(|_| ConversionError::FpStorageOffsetDoesNotFit)?;
                        Ok(UnwindRuleLoongArch64::OffsetSpAndRestoreFpAndRa {
                            sp_offset_by_16,
                            fp_storage_offset_from_sp_by_8,
                            ra_storage_offset_from_sp_by_8,
                        })
                    }
                }
            }
            LoongArch::R22 => {
                let ra_cfa_offset = register_rule_to_cfa_offset(ra_rule)?
                    .ok_or(ConversionError::FramePointerRuleDoesNotRestoreRa)?;
                let fp_cfa_offset = register_rule_to_cfa_offset(fp_rule)?
                    .ok_or(ConversionError::FramePointerRuleDoesNotRestoreFp)?;
                if *offset == 0 && fp_cfa_offset == -16 && ra_cfa_offset == -8 {
                    Ok(UnwindRuleLoongArch64::UseFramePointer)
                } else {
                    let sp_offset_from_fp_by_8 = u16::try_from(offset / 8)
                        .map_err(|_| ConversionError::SpOffsetFromFpDoesNotFit)?;
                    let ra_storage_offset_from_fp_by_8 =
                        i16::try_from((offset + ra_cfa_offset) / 8)
                            .map_err(|_| ConversionError::RaStorageOffsetDoesNotFit)?;
                    let fp_storage_offset_from_fp_by_8 =
                        i16::try_from((offset + fp_cfa_offset) / 8)
                            .map_err(|_| ConversionError::FpStorageOffsetDoesNotFit)?;
                    Ok(UnwindRuleLoongArch64::UseFramepointerWithOffsets {
                        sp_offset_from_fp_by_8,
                        fp_storage_offset_from_fp_by_8,
                        ra_storage_offset_from_fp_by_8,
                    })
                }
            }
            _ => Err(ConversionError::CfaIsOffsetFromUnknownRegister),
        },
        CfaRule::Expression(_) => Err(ConversionError::CfaIsExpression),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_translate_frame_pointer_rule() {
        let cfa_rule = CfaRule::<usize>::RegisterAndOffset {
            register: LoongArch::R22,
            offset: 0,
        };
        let fp_rule = RegisterRule::<usize>::Offset(-16);
        let ra_rule = RegisterRule::<usize>::Offset(-8);

        assert_eq!(
            translate_into_unwind_rule(&cfa_rule, Some(&fp_rule), Some(&ra_rule)).unwrap(),
            UnwindRuleLoongArch64::UseFramePointer
        );
    }

    #[test]
    fn test_translate_stack_pointer_rule() {
        let cfa_rule = CfaRule::<usize>::RegisterAndOffset {
            register: LoongArch::R3,
            offset: 32,
        };
        let fp_rule = RegisterRule::<usize>::Offset(-16);
        let ra_rule = RegisterRule::<usize>::Offset(-8);

        assert_eq!(
            translate_into_unwind_rule(&cfa_rule, Some(&fp_rule), Some(&ra_rule)).unwrap(),
            UnwindRuleLoongArch64::OffsetSpAndRestoreFpAndRa {
                sp_offset_by_16: 2,
                fp_storage_offset_from_sp_by_8: 2,
                ra_storage_offset_from_sp_by_8: 3,
            }
        );
    }

    #[test]
    fn test_unknown_cfa_register_is_rejected() {
        let cfa_rule = CfaRule::<usize>::RegisterAndOffset {
            register: LoongArch::R4,
            offset: 0,
        };
        let fp_rule = RegisterRule::<usize>::Offset(-16);
        let ra_rule = RegisterRule::<usize>::Offset(-8);

        assert!(matches!(
            translate_into_unwind_rule(&cfa_rule, Some(&fp_rule), Some(&ra_rule)),
            Err(ConversionError::CfaIsOffsetFromUnknownRegister)
        ));
    }
}
