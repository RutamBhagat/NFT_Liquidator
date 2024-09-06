pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use error::FlashFillError;
pub use instructions::*;

declare_id!("C9Qipyu7SfDZSKDNzhvtUUpG4hcsLNLKgyt6Jq7Ymcf");

#[program]
pub mod flash_fill {
    use super::*;

    pub fn borrow(ctx: Context<Borrow>) -> Result<()> {
        instructions::borrow::borrow_handler(ctx)
    }

    pub fn repay(ctx: Context<Repay>) -> Result<()> {
        instructions::repay::repay_handler(ctx)
    }
}
