
import * as anchor from "@project-serum/anchor";
import { Connection, PublicKey, Keypair } from "@solana/web3.js";
import { getAssociatedTokenAddress,
    getOrCreateAssociatedTokenAccount,createMint ,
    TOKEN_PROGRAM_ID,getAccount} from "@solana/spl-token";
import * as fs from "fs";
import * as path from "path";
import "dotenv/config";
import { fileURLToPath } from "url";
import {  Wallet } from "@project-serum/anchor";
import { BN } from "bn.js"; // ⚡ safest — BN is actually from bn.js

// ========= Config =========
const PROGRAM_ID = new PublicKey("CEHTQCjD4A4z6MYRjXydYvFhnz6s5E5Wha8XvH9xvFXM");
const CLUSTER_URL = "https://api.devnet.solana.com";
// const CLUSTER_URL = "https://solana-devnet.rpcpool.com"; // alternative

// Get IDL
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const idlPath = path.join(__dirname, "./dco.json");
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


const tokenPrice = new BN(1000000); // example 1 ZKTC = 1_000_000 lamports- 0.001 sol
const dcoEndTime = Math.floor(Date.now() / 1000) + 86400; // 1 day later
const zcw = new PublicKey("9Sd9LLuR1LLy1MtRvvqUNTDq3arnPptRjLRmWzgM9fCz");


async function initialize(){

 const owner = wallet.publicKey
    // 3. Derive PDA accounts
    const [statePda] =  PublicKey.findProgramAddressSync(
      [Buffer.from("dco_state"), owner.toBuffer()],
      program.programId
    );
    console.log("statePda", statePda)
    
    const [vaultPda] =  PublicKey.findProgramAddressSync(
      [Buffer.from("dco_vault"), statePda.toBuffer()],
      program.programId
    );
    console.log("vaultPda",vaultPda)
    
//     const sig = await program.methods
//       .initialize(tokenPrice, new BN(dcoEndTime), zcw)
//       .accounts({
//         owner : wallet.publicKey,
//         tokenMint : mint,
//         state: statePda,
//         vault: vaultPda,
//         tokenProgram: TOKEN_PROGRAM_ID,
//         systemProgram: anchor.web3.SystemProgram.programId,
//         rent: anchor.web3.SYSVAR_RENT_PUBKEY,
//       })
//       .rpc();
    
//   console.log("✅ initialize() tx:", sig);    
//   statePda PublicKey [PublicKey(hhrZEvTvUqmnvzZGRymuXKxJpnqudCX18VvcZ6BEoSL)] {
//   _bn: <BN: a6d6d35376942d3cdeb1fb0cf929eb54632d8a3d03bfffb209d83274500e57d>
// }
// vaultPda PublicKey [PublicKey(ww2Z89doCbtzQZnfD2kXSDakmZDUMEPF1rhGcuhydX9)] {
//   _bn: <BN: e1273b4c0def8086c4855975f8f60f0d64e62bd02a1b1fc3dd11a4a831571e4>
// }
    // 4Pcd1uVffP7YHqR83ubxi8ezoq13PFL1Tqr1XrqPKahbXkswLzBpDjrNcDgiAVpu47CjC8EA43yMVUW3MjpVbErk
}    
// initialize()


async function injectSupply(){

     const owner = wallet.publicKey

    const [statePda] =  PublicKey.findProgramAddressSync(
      [Buffer.from("dco_state"), owner.toBuffer()],
      program.programId
    );
    // console.log("statePda", statePda)
    
    const [vaultPda] =  PublicKey.findProgramAddressSync(
      [Buffer.from("dco_vault"), statePda.toBuffer()],
      program.programId
    );
    const amount = new BN(100_000*1000000000);
    // 3. Call inject_supply
//  const sig = await program.methods
//   .injectSupply(amount)
//   .accounts({
//     state: statePda,
//     from: tokenAccount, //owner ATA
//     vault: vaultPda,
//     owner: wallet.publicKey,
//     tokenProgram: TOKEN_PROGRAM_ID,
//   })
//   .rpc();

//   console.log("✅ inject supply() tx:", sig); 
}
// injectSupply()

async function vaultBal(){

    const owner = wallet.publicKey
    // 3. Derive PDA accounts
    const [statePda] =  PublicKey.findProgramAddressSync(
      [Buffer.from("dco_state"), owner.toBuffer()],
      program.programId
    );
    console.log("statePda", statePda)
    
    const [vaultPda] =  PublicKey.findProgramAddressSync(
      [Buffer.from("dco_vault"), statePda.toBuffer()],
      program.programId
    );
// check vault balance
const vaultInfo = await getAccount(connection, vaultPda);

console.log("Vault Mint:", vaultInfo.mint.toBase58());
console.log("Vault Owner:", vaultInfo.owner.toBase58());
console.log("Vault Amount (raw):", vaultInfo.amount.toString());

}
// vaultBal()

async function addreleser(){
    const owner = wallet.publicKey;

// the PDA of your state (same one from initialize)
const [statePda] = PublicKey.findProgramAddressSync(
  [Buffer.from("dco_state"), owner.toBuffer()],
  program.programId
);

// the new releaser wallet address you want to add
const newReleaser = new PublicKey("DR1RUW23oNWUy4GAuXB7gzsT56PyZ8Tj3a7ZEyfDnXPW");

const sig =await program.methods
  .addReleaser(newReleaser)
  .accounts({
    state: statePda,
    owner: owner, // must sign
  })
  .rpc();
   console.log("✅ add Releaser() tx:", sig); 
   //5FWEaVKF3ryYS8JGndkYG3Zp9TULwhxZPZ3iyvryuwzRCerQWZEa8URB7GX9QpyjJVH4ZPPMrtFiNY1dLybZdstB
}
// addreleser()
/////////
async function removeReleser(){
    const owner = wallet.publicKey;

// the PDA of your state (same one from initialize)
const [statePda] = PublicKey.findProgramAddressSync(
  [Buffer.from("dco_state"), owner.toBuffer()],
  program.programId
);

// the new releaser wallet address you want to add
const oldReleaser = new PublicKey("DR1RUW23oNWUy4GAuXB7gzsT56PyZ8Tj3a7ZEyfDnXPW");

const sig =await program.methods
  .removeReleaser(oldReleaser)
  .accounts({
    state: statePda,
    owner: owner, // must sign
  })
  .rpc();
   console.log("✅ remove Releaser() tx:", sig); 
   //4buxtLjGdsKyN4XsPZrh9c8k1xD2RQYNKfrz5WqWQKjK2r1rg3irRzpr6BMxrJXZ7u1XGVQVXsViQwePFsnHkHwu
}
//  removeReleser()
////////

async function releasZktc(){

const owner = wallet.publicKey;

// State PDA
const [statePda] = PublicKey.findProgramAddressSync(
  [Buffer.from("dco_state"), owner.toBuffer()],
  program.programId
);

// Vault PDA (derived inside initialize)
const [vaultPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("dco_vault"), statePda.toBuffer()],
  program.programId
);

// Buyer’s associated token account for ZKTC (must exist)
const buyerWallet = new PublicKey("DR1RUW23oNWUy4GAuXB7gzsT56PyZ8Tj3a7ZEyfDnXPW");

const buyerTokenAccount = getOrCreateAssociatedTokenAccount(
  connection,
  wallet.payer,              // payer (for rent)
  mint,                  // mint = your pre-deployed ZKTC
  buyerWallet                // owner of the token account
);

const amount = new BN(100 * 1000000000);   // number of ZKTC tokens to release
const donationAmount = new BN(2000_000_000); // donation in lamports or fixed units

const sig =await program.methods
  .releaseZktc(amount, donationAmount)
  .accounts({
    state: statePda,
    caller: owner, // must be owner or releaser
    vault: vaultPda,
    buyerTokenAccount: (await buyerTokenAccount).address,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .rpc();

   console.log("✅  Releaser ZKTC() tx:", sig); 
//KrstBwDV8sw3MsXZWgGBRovZbRpw5JjNfu2svkamtVwKQrxNAvkNiqvP8MDrZyqGywNo5oWKDBx5RJbLrshgFis
}

// releasZktc()

////////////////
//check DCO state 
async function dcoState(){
    const owner = wallet.publicKey;

// derive state PDA
const [statePda] =PublicKey.findProgramAddressSync(
  [Buffer.from("dco_state"), owner.toBuffer()],
  program.programId
);

// fetch state account
const state = await program.account.dcoState.fetch(statePda);

const decimals =9

console.log("Token Sold:", state.tokenSold.toString());
console.log("Total Donations:", state.totalDonations.toString());

console.log("Token Sold (human):", Number(state.tokenSold) / 10 ** decimals, "ZKTC");
console.log("Total Donations (human):", Number(state.totalDonations) / 10 ** decimals, "Zktc");

}
// dcoState()

async function donateToZcw() {
  const owner = wallet.publicKey;

  // State PDA
  const [statePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("dco_state"), owner.toBuffer()],
    program.programId
  );

  // Vault PDA
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("dco_vault"), statePda.toBuffer()],
    program.programId
  );


  const zcwTokenAccount = await getOrCreateAssociatedTokenAccount(
    connection,
    wallet.payer,     // payer for rent
    mint,             // ZKTC mint
    zcw         // ZCW wallet
  );

  const sig = await program.methods
    .donateToZcw()
    .accounts({
      state: statePda,
      caller: owner,
      vault: vaultPda,
      zcwTokenAccount: zcwTokenAccount.address,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .rpc();

  console.log("✅ Donation sent to ZCW, tx:", sig);
  //3ywEpPij2ZTyCW7UHsEfJyEpMRUYGBQe8cKc8UzHGjpfdxGUWfd92zKR9PTAXW1G5KjDwYkgH5EoWavV6KLGD1dr
}

// donateToZcw()
