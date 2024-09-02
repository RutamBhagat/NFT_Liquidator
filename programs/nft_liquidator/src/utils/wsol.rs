use crate::constants::*;
use crate::error::ErrorCode;
use anchor_lang::{prelude::*, system_program};
use anchor_spl::token::{self, Mint, Token, TokenAccount};

pub fn create_wsol_token_idempotent<'info>(
    program_authority: SystemAccount<'info>,
    program_wsol_account: UncheckedAccount<'info>,
    sol_mint: Account<'info, Mint>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    authority_bump: &[u8],
    wsol_bump: &[u8],
) -> Result<TokenAccount> {
    if program_wsol_account.data_is_empty() {
        let signer_seeds: &[&[&[u8]]] = &[
            &[AUTHORITY_SEED, authority_bump.as_ref()],
            &[WSOL_SEED, wsol_bump.as_ref()],
        ];

        msg!("Initialize program wSOL account");
        let rent = Rent::get()?;
        let space = TokenAccount::LEN;
        let lamports = rent.minimum_balance(space);
        system_program::create_account(
            CpiContext::new_with_signer(
                system_program.to_account_info(),
                system_program::CreateAccount {
                    from: program_authority.to_account_info(),
                    to: program_wsol_account.to_account_info(),
                },
                signer_seeds,
            ),
            lamports,
            space as u64,
            token_program.key,
        )?;

        msg!("Initialize program wSOL token account");
        token::initialize_account3(CpiContext::new(
            token_program.to_account_info(),
            token::InitializeAccount3 {
                account: program_wsol_account.to_account_info(),
                mint: sol_mint.to_account_info(),
                authority: program_authority.to_account_info(),
            },
        ))?;

        let data = program_wsol_account.try_borrow_data()?;
        let wsol_token_account = TokenAccount::try_deserialize(&mut data.as_ref())?;

        Ok(wsol_token_account)
    } else {
        let data = program_wsol_account.try_borrow_data()?;
        let wsol_token_account = TokenAccount::try_deserialize(&mut data.as_ref())?;
        if &wsol_token_account.owner != program_authority.key {
            // TODO: throw error
            return err!(ErrorCode::IncorrectOwner);
        }

        Ok(wsol_token_account)
    }
}

pub fn close_program_wsol<'info>(
    program_authority: SystemAccount<'info>,
    program_wsol_account: UncheckedAccount<'info>,
    token_program: Program<'info, Token>,
    authority_bump: &[u8],
) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[AUTHORITY_SEED, authority_bump.as_ref()]];

    msg!("Close program wSOL token account");
    token::close_account(CpiContext::new_with_signer(
        token_program.to_account_info(),
        token::CloseAccount {
            account: program_wsol_account.to_account_info(),
            destination: program_authority.to_account_info(),
            authority: program_authority.to_account_info(),
        },
        signer_seeds,
    ))
}
