use crate::constants::*;
use crate::state::Jupiter;
use crate::utils::{jupiter_funcs, wsol};
use anchor_lang::{prelude::*, system_program};
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
pub struct SwapToSOL<'info> {
    #[account(mut, seeds = [AUTHORITY_SEED], bump)]
    pub program_authority: SystemAccount<'info>,
    /// CHECK: This may not be initialized yet.
    #[account(mut, seeds = [WSOL_SEED], bump)]
    pub program_wsol_account: UncheckedAccount<'info>,
    pub user_account: Signer<'info>,
    #[account(address = spl_token::native_mint::id())]
    pub sol_mint: Account<'info, Mint>,
    pub jupiter_program: Program<'info, Jupiter>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> SwapToSOL<'info> {
    pub fn swap_to_sol(&self, bumps: &SwapToSOLBumps, data: Vec<u8>) -> Result<()> {
        let authority_bump = [bumps.program_authority];
        let wsol_bump = [bumps.program_wsol_account];

        wsol::create_wsol_token_idempotent(
            self.program_authority.clone(),
            self.program_wsol_account.clone(),
            self.sol_mint.clone(),
            self.token_program.clone(),
            self.system_program.clone(),
            &authority_bump,
            &wsol_bump,
        )?;

        msg!("Swap on Jupiter");
        let remaining_accounts: Vec<AccountInfo<'info>> = self.to_account_infos();
        jupiter_funcs::swap_on_jupiter(&remaining_accounts, self.jupiter_program.clone(), data)?;

        let after_swap_lamports = self.program_wsol_account.lamports();

        wsol::close_program_wsol(
            self.program_authority.clone(),
            self.program_wsol_account.clone(),
            self.token_program.clone(),
            &authority_bump,
        )?;

        let rent = Rent::get()?;
        let space = TokenAccount::LEN;
        let token_lamports = rent.minimum_balance(space);
        let out_amount = after_swap_lamports - token_lamports;

        msg!("Transfer SOL to user");
        let signer_seeds: &[&[&[u8]]] = &[&[AUTHORITY_SEED, authority_bump.as_ref()]];
        let lamports = out_amount;
        system_program::transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                system_program::Transfer {
                    from: self.program_authority.to_account_info(),
                    to: self.user_account.to_account_info(),
                },
                signer_seeds,
            ),
            lamports,
        )?;

        Ok(())
    }
}
