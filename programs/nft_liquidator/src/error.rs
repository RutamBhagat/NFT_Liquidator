use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid return data")]
    InvalidReturnData,
    #[msg("Invalid Jupiter program")]
    InvalidJupiterProgram,
    #[msg("Invalid Jupiter route")]
    IncorrectOwner,
}
