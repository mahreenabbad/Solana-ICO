import {
  Connection,
  PublicKey,
  Transaction,
  sendAndConfirmTransaction,
  Keypair,
} from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";

import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";

import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";
import { Wallet } from "@project-serum/anchor";

const PROGRAM_ID = new PublicKey("kLtacENmn3CG5rdYXjKQNS8ZR9Be99GniTCdLKW4U75");
const CLUSTER_URL = "https://api.devnet.solana.com";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const idlPath = path.join(__dirname, "./marketplace.json");
const idl = JSON.parse(fs.readFileSync(idlPath, "utf-8"));

const secretKeyPath = path.join(__dirname, "../new_program-keypair.json");
const secretKey = Uint8Array.from(
  JSON.parse(fs.readFileSync(secretKeyPath, "utf8"))
);
const payer = Keypair.fromSecretKey(secretKey);
const wallet = new Wallet(payer);

const connection = new Connection(CLUSTER_URL, "confirmed");
const provider = new anchor.AnchorProvider(connection, wallet, {
  preflightCommitment: "confirmed",
});
anchor.setProvider(provider);
const program = new anchor.Program(idl, PROGRAM_ID, provider);

const mint = new PublicKey("CFSa2tjMU8Qkgt8LChdXRaU4rNGmM1yBs1dhbwzm17Mc");
const treasuryPda = new PublicKey("wyM2rycgTQVN8mDNSbdE22pfc4jxbq35hCEiZubc3pV");

(async () => {
  // Derive ATA for PDA
  const treasuryATA = await getAssociatedTokenAddress(
    mint,
    treasuryPda,
    true // allowOwnerOffCurve (needed for PDA)
  );

  console.log("Treasury ATA:", treasuryATA.toBase58());

  // Instruction to create ATA
  const ix = createAssociatedTokenAccountInstruction(
    payer.publicKey, // who pays the rent
    treasuryATA, // ATA address
    treasuryPda, // owner of the ATA (PDA)
    mint
  );

  // Build & send transaction
  const tx = new Transaction().add(ix);

  const sig = await sendAndConfirmTransaction(connection, tx, [payer]);
  console.log("✅ Treasury ATA created:", treasuryATA.toBase58());
  console.log("Tx signature:", sig);
})();
