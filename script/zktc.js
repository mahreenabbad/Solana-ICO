

import * as anchor from "@project-serum/anchor";
import { Connection, PublicKey, Keypair } from "@solana/web3.js";
import { getAssociatedTokenAddress,getOrCreateAssociatedTokenAccount,createMint ,TOKEN_PROGRAM_ID} from "@solana/spl-token";
import * as fs from "fs";
import * as path from "path";
import "dotenv/config";
import { fileURLToPath } from "url";
import {  Wallet } from "@project-serum/anchor";
import { BN } from "bn.js"; // ⚡ safest — BN is actually from bn.js

// ========= Config =========
const PROGRAM_ID = new PublicKey("9kV35dMKi9azmFAiibSUkyMFoZ3QikfE9BV12dN6sgaK");
const CLUSTER_URL = "https://api.devnet.solana.com";
// const CLUSTER_URL = "https://solana-devnet.rpcpool.com"; // alternative

// Get IDL
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const idlPath = path.join(__dirname, "./zktc.json");
const idl = JSON.parse(fs.readFileSync(idlPath, "utf-8"));
// import idl from '../target/idl/zktc.json' assert {type : "json"};
if (!idl.version || !idl.instructions || !idl.accounts) {
  throw new Error("❌ IDL is invalid: missing version, instructions, or accounts");
}

console.log("✅ IDL is valid, version:", idl.version);

// Load wallet
const secretKeyPath = path.join(__dirname, "../new_program-keypair.json");
const secretKeyString = fs.readFileSync(secretKeyPath, "utf8");
const secretKey = Uint8Array.from(JSON.parse(secretKeyString));
const payer = Keypair.fromSecretKey(secretKey);
const wallet = new Wallet(payer);

// ========= Provider / Program =========
const connection = new Connection(CLUSTER_URL, "confirmed");
const provider = new anchor.AnchorProvider(connection, wallet, { 
  preflightCommitment: "confirmed" 
});
anchor.setProvider(provider);
const program = new anchor.Program(idl, PROGRAM_ID, provider);
const mint = new PublicKey("Fi7ZwQ3wHDTFDAn5knBrGQ8yMqhw5vgFM9FGFjCADKRn")
const tokenAccount = new PublicKey("F4zRjkJyB3rdpqsyeaPUbpnSFN5R2FCYGZEBXHDHen8g")
const state = new PublicKey("ANtuQCRHCVUBhuesb5of9YeuQSLLBw7epwCPZTNxwGKX")
async function main() {
  // authority (Phantom wallet)

  // Create a new mint
//   const mint = await createMint(
//     connection,      // connection
//     payer,    // fee payer (Phantom)
//     wallet.publicKey,                // mint authority (who can mint new tokens)
//     wallet.publicKey,                     // if freeze authority (null = no freeze)
//     9                         // decimals (like 9 for SOL, 6 for USDC)
//   );

//   console.log("✅ New Mint Created:", mint);

   // Optional: create ATA (token account for authority to hold tokens)
//   const tokenAccount = await getOrCreateAssociatedTokenAccount(
//     connection,
//     payer,
//     mint,
//     wallet.publicKey
//   );

//   console.log("✅ Token Account:", tokenAccount.address.toBase58());
// const tokenAccount = F4zRjkJyB3rdpqsyeaPUbpnSFN5R2FCYGZEBXHDHen8g




}

//  main()

// console.log(stateKeypair.publicKey.toBase58()); // ✅ works
// console.log(stateKeypair.secretKey)
async function initialize() {
  const stateKeypair = Keypair.generate();

  console.log("State account:", stateKeypair.publicKey.toBase58());
 const sig = await program.methods
      .initialize()
      .accounts({
        state: stateKeypair.publicKey, //only owner
        mint: mint,
        authority: wallet.publicKey,
        system_program:  anchor.web3.SystemProgram.programId,
        token_program: TOKEN_PROGRAM_ID
      })
      .signers([stateKeypair]) 
      .rpc();
    
    console.log("✅ initialize() tx:", sig);
//     State account: ANtuQCRHCVUBhuesb5of9YeuQSLLBw7epwCPZTNxwGKX
// ✅ initialize() tx: 4AtDBYPX8hfuhuHr3ybj1rZZckfebB9muCob6FrBCMUApRep5wN1GrQbUK35dUFV6XGBx5oBEPw9QngLsRH26cwF

}
// initialize()



async function mintFun() {
  // Derive ATA for wallet + mint
//   const tokenAccount = await getAssociatedTokenAddress(
//     mint,
//     wallet.publicKey
//   );
//   console.log("tokenAccount:", tokenAccount.toBase58());

  // Ensure ATA exists
  // const walletadd =new PublicKey("CRb4bz1HSNaGtZX6qxuFrCYfnpND9cDu5ey2REqZuLNE")
  try {
  
    // Mint tokens
    const mintAmount = new BN("100000000000000"); 
  const mintSig = await program.methods
    .mint(mintAmount)
    .accounts({
      state: state,
      mint: mint,
      tokenAccount: tokenAccount,   // ✅ pass ATA pubkey
      tokenProgram: TOKEN_PROGRAM_ID,
      authority: wallet.publicKey,
    })
    .rpc();
    
    console.log("✅ mint() tx:", mintSig);
    // 57F35D4bqiHQZQ3BoY6WtLoKnDaM7o24AWQr6j9LUmD4ATHkjGZ6cFKbjxsW5wddXhfhWjJkwVWW5sBmgaThSLur
    // 4DxoqdLBpSQ6zNT3dP3KN1izNyyhPktFvHKmNhkhVapocNAUfADCRzsviGKCArXPV48oKJBRMpK2QUJFbv9dm69Y
  } catch (e) {
    if (e.message.includes("already in use")) {
      console.log("ℹ️ ATA already exists:", tokenAccount.toBase58());
    } else {
      throw e;
    }
  }}
//  mintFun()



async function burn() {
  // 1️⃣ Get the wallet's ATA for this mint
//   const tokenAccount = await getAssociatedTokenAddress(
//     mint,
//     wallet.publicKey
//   );
  // console.log("tokenAccount (for burn):", tokenAccount.toBase58());

  // 2️⃣ Amount to burn (e.g. burn 500 tokens)
  const burnAmount = new BN(50 * 10 ** 9); // adjust decimals

  // 3️⃣ Call burn method
  const burnSig = await program.methods
    .burn(burnAmount)
    .accounts({
      state: state, // state PDA / account
      mint: mint,
      tokenAccount: tokenAccount,
      tokenProgram: TOKEN_PROGRAM_ID,
      authority: wallet.publicKey,
    })
    .rpc();

  console.log("🔥 burn() tx:", burnSig);
//   🔥 burn() tx: 4AMNTwUYv6z74136bytvf8brL2o14y4UVKwiPjCPujVWvgSP2bTrRUFhxRV3cQMYpoYrqnkvT8MQvvrqRK3BtKve
}
// burn()
