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

pub use constants::*;
pub use error::ErrorCode;
pub use instructions::*;
pub use state::*;

declare_id!("S5yb3Bysioeuq7pvuixejRTVfu1kEFUm5gzmkLJL5Dc");

#[program]
pub mod nft_liquidator {
    use super::*;

    pub fn swap_to_sol(ctx: Context<SwapToSOL>, data: Vec<u8>) -> Result<()> {
        let authority_bump = [ctx.bumps.program_authority];
        let wsol_bump = [ctx.bumps.program_wsol_account];

        create_wsol_token_idempotent(
            ctx.accounts.program_authority.clone(),
            ctx.accounts.program_wsol_account.clone(),
            ctx.accounts.sol_mint.clone(),
            ctx.accounts.token_program.clone(),
            ctx.accounts.system_program.clone(),
            &authority_bump,
            &wsol_bump,
        )?;

        msg!("Swap on Jupiter");
        swap_on_jupiter(
            ctx.remaining_accounts,
            ctx.accounts.jupiter_program.clone(),
            data,
        )?;

        let after_swap_lamports = ctx.accounts.program_wsol_account.lamports();

        close_program_wsol(
            ctx.accounts.program_authority.clone(),
            ctx.accounts.program_wsol_account.clone(),
            ctx.accounts.token_program.clone(),
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
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.program_authority.to_account_info(),
                    to: ctx.accounts.user_account.to_account_info(),
                },
                signer_seeds,
            ),
            lamports,
        )?;

        Ok(())
    }
}

fn swap_on_jupiter<'info>(
    remaining_accounts: &[AccountInfo],
    jupiter_program: Program<'info, Jupiter>,
    data: Vec<u8>,
) -> ProgramResult {
    let accounts: Vec<AccountMeta> = remaining_accounts
        .iter()
        .map(|acc| AccountMeta {
            pubkey: *acc.key,
            is_signer: acc.is_signer,
            is_writable: acc.is_writable,
        })
        .collect();

    let accounts_infos: Vec<AccountInfo> = remaining_accounts
        .iter()
        .map(|acc| AccountInfo { ..acc.clone() })
        .collect();

    // TODO: Check the first 8 bytes. Only Jupiter Route CPI allowed.

    invoke_signed(
        &Instruction {
            program_id: *jupiter_program.key,
            accounts,
            data,
        },
        &accounts_infos,
        &[],
    )
}

fn create_wsol_token_idempotent<'info>(
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

fn close_program_wsol<'info>(
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
