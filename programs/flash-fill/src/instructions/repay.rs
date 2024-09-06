use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar;
use anchor_lang::solana_program::sysvar::instructions::{
    load_current_index_checked, load_instruction_at_checked,
};
use anchor_lang::system_program;
use anchor_spl::token::TokenAccount;

use crate::constants::*;
use crate::error::FlashFillError;

#[derive(Accounts)]
pub struct Repay<'info> {
    pub borrower: Signer<'info>,
    #[account(mut, seeds = [AUTHORITY_SEED], bump)]
    pub program_authority: SystemAccount<'info>,
    /// CHECK: check instructions account
    #[account(address = sysvar::instructions::ID @FlashFillError::AddressMismatch)]
    pub instructions: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn repay_handler(ctx: Context<Repay>) -> Result<()> {
    let ixs = ctx.accounts.instructions.to_account_info();

    // make sure this isnt a cpi call
    let current_index = load_current_index_checked(&ixs)? as usize;
    let current_ix = load_instruction_at_checked(current_index, &ixs)?;
    if current_ix.program_id != *ctx.program_id {
        return Err(FlashFillError::ProgramMismatch.into());
    }

    let rent = Rent::get()?;
    let space = TokenAccount::LEN;
    let token_lamports = rent.minimum_balance(space);

    // transfer borrowed SOL back to the program authority
    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.borrower.to_account_info(),
                to: ctx.accounts.program_authority.to_account_info(),
            },
        ),
        token_lamports,
    )?;

    Ok(())
}
