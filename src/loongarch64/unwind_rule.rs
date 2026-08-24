use super::unwindregs::UnwindRegsLoongArch64;
use crate::add_signed::checked_add_signed;
use crate::error::Error;
use crate::unwind_rule::UnwindRule;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnwindRuleLoongArch64 {
    /// (sp, fp, ra) = (sp, fp, ra)
    /// Only possible for the first frame. Subsequent frames must get the
    /// return address from somewhere other than the ra register to avoid
    /// infinite loops.
    NoOp,
    /// (sp, fp, ra) = if is_first_frame (sp, fp, ra) else (fp, *(fp - 16), *(fp - 8))
    /// Used as a fallback rule.
    NoOpIfFirstFrameOtherwiseFp,
    /// (sp, fp, ra) = (sp + 16x, fp, ra)
    /// Only possible for the first frame. Subsequent frames must get the
    /// return address from somewhere other than the ra register to avoid
    /// infinite loops.
    OffsetSp { sp_offset_by_16: u16 },
    /// (sp, fp, ra) = (sp + 16x, fp, ra) if is_first_frame
    /// This rule reflects an ambiguity in DWARF CFI information. When the
    /// return address is "undefined" because it was omitted, it could mean
    /// "same value", but this is only allowed for the first frame.
    OffsetSpIfFirstFrameOtherwiseStackEndsHere { sp_offset_by_16: u16 },
    /// (sp, fp, ra) = (sp + 16x, fp, *(sp + 8y))
    OffsetSpAndRestoreRa {
        sp_offset_by_16: u16,
        ra_storage_offset_from_sp_by_8: i16,
    },
    /// (sp, fp, ra) = (sp + 16x, *(sp + 8y), *(sp + 8z))
    OffsetSpAndRestoreFpAndRa {
        sp_offset_by_16: u16,
        fp_storage_offset_from_sp_by_8: i16,
        ra_storage_offset_from_sp_by_8: i16,
    },
    /// (sp, fp, ra) = (fp, *(fp - 16), *(fp - 8))
    UseFramePointer,
    /// (sp, fp, ra) = (fp + 8x, *(fp + 8y), *(fp + 8z))
    UseFramepointerWithOffsets {
        sp_offset_from_fp_by_8: u16,
        fp_storage_offset_from_fp_by_8: i16,
        ra_storage_offset_from_fp_by_8: i16,
    },
}

impl UnwindRule for UnwindRuleLoongArch64 {
    type UnwindRegs = UnwindRegsLoongArch64;

    fn rule_for_stub_functions() -> Self {
        UnwindRuleLoongArch64::NoOp
    }
    fn rule_for_function_start() -> Self {
        UnwindRuleLoongArch64::NoOp
    }
    fn fallback_rule() -> Self {
        UnwindRuleLoongArch64::UseFramePointer
    }

    fn exec<F>(
        self,
        is_first_frame: bool,
        regs: &mut UnwindRegsLoongArch64,
        read_stack: &mut F,
    ) -> Result<Option<u64>, Error>
    where
        F: FnMut(u64) -> Result<u64, ()>,
    {
        let ra = regs.ra();
        let sp = regs.sp();
        let fp = regs.fp();

        let (new_ra, new_sp, new_fp) = match self {
            UnwindRuleLoongArch64::NoOp => {
                if !is_first_frame {
                    return Err(Error::DidNotAdvance);
                }
                (ra, sp, fp)
            }
            UnwindRuleLoongArch64::NoOpIfFirstFrameOtherwiseFp => {
                if is_first_frame {
                    (ra, sp, fp)
                } else {
                    let fp = regs.fp();
                    let new_sp = fp;
                    if new_sp <= sp {
                        return Err(Error::FramepointerUnwindingMovedBackwards);
                    }
                    let ra_location = fp.checked_sub(8).ok_or(Error::IntegerOverflow)?;
                    let fp_location = fp.checked_sub(16).ok_or(Error::IntegerOverflow)?;
                    let new_ra = read_stack(ra_location)
                        .map_err(|_| Error::CouldNotReadStack(ra_location))?;
                    let new_fp = read_stack(fp_location)
                        .map_err(|_| Error::CouldNotReadStack(fp_location))?;
                    (new_ra, new_sp, new_fp)
                }
            }
            UnwindRuleLoongArch64::OffsetSpIfFirstFrameOtherwiseStackEndsHere {
                sp_offset_by_16,
            } => {
                if !is_first_frame {
                    return Ok(None);
                }
                let sp_offset = u64::from(sp_offset_by_16) * 16;
                let new_sp = sp.checked_add(sp_offset).ok_or(Error::IntegerOverflow)?;
                (ra, new_sp, fp)
            }
            UnwindRuleLoongArch64::OffsetSp { sp_offset_by_16 } => {
                if !is_first_frame {
                    return Err(Error::DidNotAdvance);
                }
                let sp_offset = u64::from(sp_offset_by_16) * 16;
                let new_sp = sp.checked_add(sp_offset).ok_or(Error::IntegerOverflow)?;
                (ra, new_sp, fp)
            }
            UnwindRuleLoongArch64::OffsetSpAndRestoreRa {
                sp_offset_by_16,
                ra_storage_offset_from_sp_by_8,
            } => {
                let sp_offset = u64::from(sp_offset_by_16) * 16;
                let new_sp = sp.checked_add(sp_offset).ok_or(Error::IntegerOverflow)?;
                let ra_storage_offset = i64::from(ra_storage_offset_from_sp_by_8) * 8;
                let ra_location =
                    checked_add_signed(sp, ra_storage_offset).ok_or(Error::IntegerOverflow)?;
                let new_ra =
                    read_stack(ra_location).map_err(|_| Error::CouldNotReadStack(ra_location))?;
                (new_ra, new_sp, fp)
            }
            UnwindRuleLoongArch64::OffsetSpAndRestoreFpAndRa {
                sp_offset_by_16,
                fp_storage_offset_from_sp_by_8,
                ra_storage_offset_from_sp_by_8,
            } => {
                let sp_offset = u64::from(sp_offset_by_16) * 16;
                let new_sp = sp.checked_add(sp_offset).ok_or(Error::IntegerOverflow)?;
                let ra_storage_offset = i64::from(ra_storage_offset_from_sp_by_8) * 8;
                let ra_location =
                    checked_add_signed(sp, ra_storage_offset).ok_or(Error::IntegerOverflow)?;
                let new_ra =
                    read_stack(ra_location).map_err(|_| Error::CouldNotReadStack(ra_location))?;
                let fp_storage_offset = i64::from(fp_storage_offset_from_sp_by_8) * 8;
                let fp_location =
                    checked_add_signed(sp, fp_storage_offset).ok_or(Error::IntegerOverflow)?;
                let new_fp =
                    read_stack(fp_location).map_err(|_| Error::CouldNotReadStack(fp_location))?;
                (new_ra, new_sp, new_fp)
            }
            UnwindRuleLoongArch64::UseFramePointer => {
                // LoongArch64 frame-based functions set fp to the caller's sp after saving
                // the caller's fp and ra. Therefore the caller's fp is at fp - 16, the
                // return address is at fp - 8, and the caller's sp is fp itself.
                let fp = regs.fp();
                let new_sp = fp;
                let ra_location = fp.checked_sub(8).ok_or(Error::IntegerOverflow)?;
                let fp_location = fp.checked_sub(16).ok_or(Error::IntegerOverflow)?;
                let new_ra =
                    read_stack(ra_location).map_err(|_| Error::CouldNotReadStack(ra_location))?;
                let new_fp =
                    read_stack(fp_location).map_err(|_| Error::CouldNotReadStack(fp_location))?;
                if new_fp == 0 {
                    return Ok(None);
                }
                if new_fp <= fp || new_sp <= sp {
                    return Err(Error::FramepointerUnwindingMovedBackwards);
                }
                (new_ra, new_sp, new_fp)
            }
            UnwindRuleLoongArch64::UseFramepointerWithOffsets {
                sp_offset_from_fp_by_8,
                fp_storage_offset_from_fp_by_8,
                ra_storage_offset_from_fp_by_8,
            } => {
                let sp_offset_from_fp = u64::from(sp_offset_from_fp_by_8) * 8;
                let new_sp = fp
                    .checked_add(sp_offset_from_fp)
                    .ok_or(Error::IntegerOverflow)?;
                let ra_storage_offset = i64::from(ra_storage_offset_from_fp_by_8) * 8;
                let ra_location =
                    checked_add_signed(fp, ra_storage_offset).ok_or(Error::IntegerOverflow)?;
                let new_ra =
                    read_stack(ra_location).map_err(|_| Error::CouldNotReadStack(ra_location))?;
                let fp_storage_offset = i64::from(fp_storage_offset_from_fp_by_8) * 8;
                let fp_location =
                    checked_add_signed(fp, fp_storage_offset).ok_or(Error::IntegerOverflow)?;
                let new_fp =
                    read_stack(fp_location).map_err(|_| Error::CouldNotReadStack(fp_location))?;

                if new_fp == 0 {
                    return Ok(None);
                }
                if new_fp <= fp || new_sp <= sp {
                    return Err(Error::FramepointerUnwindingMovedBackwards);
                }
                (new_ra, new_sp, new_fp)
            }
        };

        if new_ra == 0 {
            return Ok(None);
        }
        if !is_first_frame && new_sp == sp {
            return Err(Error::DidNotAdvance);
        }
        regs.set_ra(new_ra);
        regs.set_sp(new_sp);
        regs.set_fp(new_fp);

        Ok(Some(new_ra))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_basic() {
        let stack = [
            0, 0, 0, 0, 0x50, 0x100200, 0, 0, 0x80, 0x100100, 0, 0, 0, 0, 0, 0,
        ];
        let mut read_stack = |addr| Ok(stack[(addr / 8) as usize]);
        let mut regs = UnwindRegsLoongArch64::new(0x100300, 0x10, 0x30);

        let res = UnwindRuleLoongArch64::NoOp.exec(true, &mut regs, &mut read_stack);
        assert_eq!(res, Ok(Some(0x100300)));
        assert_eq!(regs.sp(), 0x10);

        let res = UnwindRuleLoongArch64::UseFramePointer.exec(false, &mut regs, &mut read_stack);
        assert_eq!(res, Ok(Some(0x100200)));
        assert_eq!(regs.sp(), 0x30);
        assert_eq!(regs.fp(), 0x50);

        let res = UnwindRuleLoongArch64::UseFramePointer.exec(false, &mut regs, &mut read_stack);
        assert_eq!(res, Ok(Some(0x100100)));
        assert_eq!(regs.sp(), 0x50);
        assert_eq!(regs.fp(), 0x80);

        let res = UnwindRuleLoongArch64::UseFramePointer.exec(false, &mut regs, &mut read_stack);
        assert_eq!(res, Ok(None));
    }

    #[test]
    fn test_fallback_returns_last_valid_return_address() {
        let stack = [0, 0, 0, 0x100100, 0, 0, 0, 0];
        let mut read_stack = |addr| Ok(stack[(addr / 8) as usize]);
        let mut regs = UnwindRegsLoongArch64::new(0x100200, 0x10, 0x20);

        let res = UnwindRuleLoongArch64::NoOpIfFirstFrameOtherwiseFp.exec(
            false,
            &mut regs,
            &mut read_stack,
        );
        assert_eq!(res, Ok(Some(0x100100)));
        assert_eq!(regs.sp(), 0x20);
        assert_eq!(regs.fp(), 0);
    }

    #[test]
    fn test_frame_pointer_must_advance() {
        let stack = [0, 0, 0, 0, 0x20, 0x100100, 0, 0];
        let mut read_stack = |addr| Ok(stack[(addr / 8) as usize]);
        let mut regs = UnwindRegsLoongArch64::new(0x100200, 0x10, 0x30);

        let res = UnwindRuleLoongArch64::UseFramePointer.exec(false, &mut regs, &mut read_stack);
        assert_eq!(res, Err(Error::FramepointerUnwindingMovedBackwards));
    }
}
