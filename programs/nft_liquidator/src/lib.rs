use anchor_lang::{
    prelude::*,
    solana_program::{entrypoint::ProgramResult, instruction::Instruction, program::invoke_signed},
    system_program,
};
use anchor_spl::token::{self, Mint, Token, TokenAccount};

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod utils;

pub use constants::*;
pub use error::ErrorCode;
pub use instructions::*;
pub use state::*;

declare_id!("S5yb3Bysioeuq7pvuixejRTVfu1kEFUm5gzmkLJL5Dc");

#[program]
pub mod nft_liquidator {
    use super::*;

    pub fn swap_to_sol(ctx: Context<SwapToSOL>, data: Vec<u8>) -> Result<()> {
        ctx.accounts.swap_to_sol(&ctx.bumps, data)
    }
}
