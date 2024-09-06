use anchor_lang::prelude::*;

/// Errors for this program
#[error_code]
pub enum FlashFillError {
    #[msg("Address Mismatch")]
    AddressMismatch,
    #[msg("Program Mismatch")]
    ProgramMismatch,
    #[msg("Missing Repay")]
    MissingRepay,
    #[msg("Incorrect Owner")]
    IncorrectOwner,
    #[msg("Incorrect Program Authority")]
    IncorrectProgramAuthority,
    #[msg("Cannot Borrow Before Repay")]
    CannotBorrowBeforeRepay,
    #[msg("Unknown Instruction")]
    UnknownInstruction,
}
