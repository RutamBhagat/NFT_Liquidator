import * as anchor from "@coral-xyz/anchor";

import {
  PublicKey,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SystemProgram,
} from "@solana/web3.js";

import { FlashFill } from "../target/types/flash_fill";
import { NATIVE_MINT } from "@solana/spl-token";
import { Program } from "@coral-xyz/anchor";
import fetch from "node-fetch";

const API_ENDPOINT = "https://quote-api.jup.ag/v6";

describe("flash-fill", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.FlashFill as Program<FlashFill>;
  const wallet = anchor.workspace.FlashFill.provider.wallet;
  const connection = anchor.getProvider().connection;

  let programAuthority: PublicKey;

  before(async () => {
    [programAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("authority")],
      program.programId
    );
  });

  const getQuote = async (
    fromMint: PublicKey,
    toMint: PublicKey,
    amount: number
  ) => {
    const response = await fetch(
      `${API_ENDPOINT}/quote?outputMint=${toMint.toBase58()}&inputMint=${fromMint.toBase58()}&amount=${amount}&slippage=0.5`
    );
    return response.json();
  };

  const getSwapIx = async (user: PublicKey, quote: any) => {
    const data = {
      quoteResponse: quote,
      userPublicKey: user.toBase58(),
    };

    const response = await fetch(`${API_ENDPOINT}/swap-instructions`, {
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      body: JSON.stringify(data),
    });
    return response.json();
  };

  it("Performs a flash loan and swap", async () => {
    const USDC = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    const amount = 1000000; // 1 USDC

    // Get quote
    const quote = await getQuote(USDC, NATIVE_MINT, amount);
    console.log("Quote:", quote);

    // Get swap instructions
    const swapInstructions = await getSwapIx(wallet.publicKey, quote);
    console.log("Swap Instructions:", swapInstructions);

    if ("error" in swapInstructions) {
      console.error("Error in swap instructions:", swapInstructions.error);
      return;
    }

    const {
      computeBudgetInstructions,
      setupInstructions,
      swapInstruction,
      cleanupInstruction,
      addressLookupTableAddresses,
    } = swapInstructions;

    // Create transaction
    const tx = new anchor.web3.Transaction();

    // Add compute budget instructions
    computeBudgetInstructions.forEach((ix) => {
      tx.add(
        anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
          units: ix.params.units,
        })
      );
    });

    // Add borrow instruction
    tx.add(
      await program.methods
        .borrow()
        .accountsPartial({
          borrower: wallet.publicKey,
          programAuthority,
          instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
          systemProgram: SystemProgram.programId,
        })
        .instruction()
    );

    // Add setup instructions
    setupInstructions.forEach((ix) => {
      tx.add(new anchor.web3.TransactionInstruction(ix));
    });

    // Add swap instruction
    tx.add(new anchor.web3.TransactionInstruction(swapInstruction));

    // Add cleanup instruction if present
    if (cleanupInstruction) {
      tx.add(new anchor.web3.TransactionInstruction(cleanupInstruction));
    }

    // Add repay instruction
    tx.add(
      await program.methods
        .repay()
        .accountsPartial({
          borrower: wallet.publicKey,
          programAuthority,
          instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
          systemProgram: SystemProgram.programId,
        })
        .instruction()
    );

    // Send and confirm transaction
    try {
      const txId = await anchor.web3.sendAndConfirmTransaction(connection, tx, [
        wallet.payer,
      ]);
      console.log("Transaction signature:", txId);
    } catch (error) {
      console.error("Error executing transaction:", error);
    }
  });
});
