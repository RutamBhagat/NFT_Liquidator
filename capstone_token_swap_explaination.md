## Overview of Jupiter Swap and Flash Fill

Jupiter Swap is a protocol built on the Solana blockchain that aggregates liquidity from various decentralized exchanges (DEXs) to provide optimal swap rates. To enhance functionality, the Flash Fill approach addresses the limitations of Cross Program Invocation (CPI), enabling more complex swaps while adhering to Solana's transaction size constraints.

## SPL Token to SOL Swap Process

The process for swapping an SPL token, such as USDC, to SOL involves several key steps:

### a) Borrowing SOL

```rust
pub fn borrow(ctx: Context<Borrow>) -> Result<()> {
    let ixs = ctx.accounts.instructions.to_account_info();

    // Ensure this is not a CPI call
    let current_index = load_current_index_checked(&ixs)? as usize;
    let current_ix = load_instruction_at_checked(current_index, &ixs)?;
    if current_ix.program_id != *ctx.program_id {
        return Err(FlashFillError::ProgramMismatch.into());
    }

    // Loop through instructions to find a corresponding repay
    let mut index = current_index + 1; // jupiter swap
    loop {
        if let Ok(ix) = load_instruction_at_checked(index, &ixs) {
            if ix.program_id == crate::id() {
                let ix_discriminator: [u8; 8] = ix.data[0..8]
                    .try_into()
                    .map_err(|_| FlashFillError::UnknownInstruction)?;

                // Check for repay instruction
                if ix_discriminator == self::instruction::Repay::discriminator() {
                    require_keys_eq!(
                        ix.accounts[1].pubkey,
                        ctx.accounts.program_authority.key(),
                        FlashFillError::IncorrectProgramAuthority
                    );

                    break;
                } else if ix_discriminator == self::instruction::Borrow::discriminator() {
                    return Err(FlashFillError::CannotBorrowBeforeRepay.into());
                } else {
                    return Err(FlashFillError::UnknownInstruction.into());
                }
            }
        } else {
            return Err(FlashFillError::MissingRepay.into());
        }

        index += 1
    }

    let authority_bump = ctx.bumps.program_authority.to_le_bytes();
    let rent = Rent::get()?;
    let space = TokenAccount::LEN;
    let token_lamports = rent.minimum_balance(space);

    // Transfer SOL to the borrower to open a wSOL account
    let signer_seeds: &[&[&[u8]]] = &[&[AUTHORITY_SEED, authority_bump.as_ref()]];
    system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.program_authority.to_account_info(),
                to: ctx.accounts.borrower.to_account_info(),
            },
            signer_seeds,
        ),
        token_lamports,
    )?;

    Ok(())
}
```

This function borrows just enough SOL to create a Wrapped SOL (wSOL) account, ensuring a corresponding repay instruction exists later in the transaction.

### b) Creating wSOL Account

The borrowed SOL is used to create a wSOL account for the user, as Jupiter Swap operates on SPL tokens, necessitating that SOL be wrapped as wSOL for the swap.

### c) Swapping SPL Token to wSOL

Jupiter Swap executes the actual swap from the input SPL token (e.g., USDC) to wSOL, interacting with various DEXs to secure the best rate.

### d) Closing wSOL Account

After the swap, the wSOL account is closed, converting the wSOL back to native SOL.

### e) Repaying Borrowed SOL

Finally, the borrowed SOL used to create the wSOL account is repaid to the program.

## SOL to SPL Token Swap Process

The process for swapping SOL to an SPL token mirrors the previous steps but in reverse:

### a) Wrapping SOL

The user's SOL is wrapped into wSOL by creating a wSOL account and depositing the SOL.

### b) Swapping wSOL to SPL Token

Jupiter Swap performs the swap from wSOL to the desired SPL token.

### c) Closing wSOL Account

If any wSOL remains, the account is closed, and the SOL is returned to the user.

## Jupiter Swap Mechanism

Jupiter Swap does not hold or transfer tokens directly. Instead, it orchestrates a series of swaps across multiple DEXs to achieve the best rates, potentially splitting the swap into multiple steps, such as:

- USDC → RAY → SOL

Each step interacts with specific DEXs (e.g., Raydium, Serum, Orca) where the trading pair has the best liquidity and pricing.

## Flash Fill Approach

The Flash Fill approach facilitates more complex transactions by utilizing Versioned Transactions and Address Lookup Tables, which reduces transaction size and allows for more operations within Solana's transaction limits.

```typescript
const swapToSol = async (
  computeBudgetPayloads: any[],
  setupPayloads: any[],
  swapPayload: any,
  cleanupPayload: any | null,
  addressLookupTableAddresses: string[]
) => {
  const instructions = [
    ...computeBudgetPayloads.map(instructionDataToTransactionInstruction),
    await program.methods
      .borrow()
      .accountsStrict({
        borrower: wallet.publicKey,
        programAuthority,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        systemProgram: SystemProgram.programId,
      })
      .instruction(),
    ...setupPayloads.map(instructionDataToTransactionInstruction),
    instructionDataToTransactionInstruction(swapPayload),
    instructionDataToTransactionInstruction(cleanupPayload), // can be null
    await program.methods
      .repay()
      .accountsStrict({
        borrower: wallet.publicKey,
        programAuthority,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        systemProgram: SystemProgram.programId,
      })
      .instruction(),
  ].filter((instruction) => instruction !== null);

  const blockhash = (await connection.getLatestBlockhash()).blockhash;

  const addressLookupTableAccounts = await getAdressLookupTableAccounts(
    addressLookupTableAddresses
  );
  const messageV0 = new TransactionMessage({
    payerKey: wallet.publicKey,
    recentBlockhash: blockhash,
    instructions,
  }).compileToV0Message(addressLookupTableAccounts);
  const transaction = new VersionedTransaction(messageV0);

  try {
    await provider.simulate(transaction, [wallet.payer]);
    const txID = await provider.sendAndConfirm(transaction, [wallet.payer]);
    console.log({ txID });
  } catch (e) {
    console.log({ simulationResponse: e.simulationResponse });
  }
};
```

This function assembles all necessary instructions for the swap, including borrowing SOL, setting up accounts, performing the swap, cleaning up, and repaying the borrowed SOL.

In summary, the process of swapping between SPL tokens and SOL involves wrapping/unwrapping SOL, executing swaps across multiple DEXs, and managing temporary SOL loans to facilitate the process. The Flash Fill approach enables these complex operations to be performed within Solana's transaction limits.
