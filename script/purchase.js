
import * as anchor from "@project-serum/anchor";
import { Connection, PublicKey, Keypair ,
      SystemProgram,
     Transaction,
    sendAndConfirmTransaction,
} from "@solana/web3.js";
import { getAssociatedTokenAddress,
        getOrCreateAssociatedTokenAccount,createMint ,
        TOKEN_PROGRAM_ID,getAccount,
        getAssociatedTokenAddressSync,
        createInitializeAccountInstruction,
        createAssociatedTokenAccountInstruction,
        getMinimumBalanceForRentExemptAccount,
    AccountLayout,} from "@solana/spl-token";
import * as fs from "fs";
import * as path from "path";
import "dotenv/config";
import { fileURLToPath } from "url";
import {  Wallet } from "@project-serum/anchor";
import { BN } from "bn.js"; // ⚡ safest — BN is actually from bn.js

// ========= Config =========
const PROGRAM_ID = new PublicKey("76sgJHKdFqnRQ4sheZzjbNFdqFKvS4akdTbeAhsNB3BA");
const CLUSTER_URL = "https://api.devnet.solana.com";
// const CLUSTER_URL = "https://solana-devnet.rpcpool.com"; // alternative

// Get IDL
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const idlPath = path.join(__dirname, "./marketplace.json");
const idl = JSON.parse(fs.readFileSync(idlPath, "utf-8"));

// import idl from '../target/idl/zktc.json' assert {type : "json"};
if (!idl.version || !idl.instructions || !idl.accounts) {
  throw new Error("❌ IDL is invalid: missing version, instructions, or accounts");
}

console.log("✅ IDL is valid, version:", idl.version);

// Load wallet
const secretKeyPath = path.join(__dirname, "../new4_program-keypair.json");

// const secretKeyPath = path.join(__dirname, "../new_program-keypair.json");
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


const makerPubkey = new PublicKey("CRb4bz1HSNaGtZX6qxuFrCYfnpND9cDu5ey2REqZuLNE"); // 👈 seller wallet
const taker = payer; // buyer

const name = "MyMarket";       // <= must be < 33 chars
const nftMint = new PublicKey("CFSa2tjMU8Qkgt8LChdXRaU4rNGmM1yBs1dhbwzm17Mc"); // the NFT being listed
// Derive PDA addresses (must match seeds in your Rust)
const [marketplacePda] = PublicKey.findProgramAddressSync(
  [Buffer.from("marketplace"), Buffer.from(name)],
  program.programId
);
const [listingPda] = PublicKey.findProgramAddressSync(
  [marketplacePda.toBuffer(), nftMint.toBuffer()],
   program.programId
);

const [rewardsMintPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("rewards"), marketplacePda.toBuffer()],
  program.programId
);

const [treasuryPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("treasury"), marketplacePda.toBuffer()],
  program.programId
);
// const makerAta = getAssociatedTokenAddressSync(nftMint, wallet.publicKey);
// console.log("maker ATA",makerAta)


// // console.log("Taker ATA:", takerAta.address.toBase58());

// const vaultAta = await getAssociatedTokenAddress(
//   nftMint,
//   listingPda,   // 👈 your program seeds vault with listing PDA as authority
//   true          // allow owner off curve
//   );
  
//   console.log("Vault ATA (NFT stored during listing):", vaultAta.toBase58());

//     console.log("Treasury ATA:", treasuryAta.address.toBase58());
const  makerATA =new PublicKey("EUcwt15546fposzhRpwqDFNJXpEnqhmhBn64hteKAEpv")
//   _bn: <BN: c83b41a22365833a49ca3a7e5e5384b3acd1945d72a19ae63d0981edf2b35817>
// }
const VaultATA  =new PublicKey("BKvN4i46gXgSxov3raEX8qAycVvGWiZfwg6T9CPujcGZ"); 
// const TreasuryATA=new PublicKey("8uVRwNDY1zE9We2miD4SUVVeXxePbErCMUpCBpxCK7zF");
const TreasuryPDA =new PublicKey("wyM2rycgTQVN8mDNSbdE22pfc4jxbq35hCEiZubc3pV");

(async () => {
    
      const treasuryAta = await getOrCreateAssociatedTokenAccount(
        connection,
        payer,             // payer for creation
        nftMint,        // the SPL token mint used for treasury
        TreasuryPDA,       // treasury PDA as owner
        true               // allow off curve
        )
        console.log("treasuryATA",treasuryAta)
    // 1. Create/get ATA for taker (buyer)
      const takerAta = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,            // payer for rent if needed
      nftMint,               // mint of NFT being purchased
      payer.publicKey,                   // owner of ATA (buyer)
      true                     // allow owner off curve (for PDA if needed)
      );
  const tx = await program.methods
    .purchase()
    .accounts({
      taker: wallet.publicKey,
      maker: makerPubkey,
      makerMint:nftMint,
      marketplace: marketplacePda,
      takerAta: takerAta.address,
      vault: VaultATA,
      rewards: rewardsMintPda,   // still required in your struct
      listing: listingPda,
      treasury: treasuryAta.address,
      associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
      tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
    })
    .signers([taker])
    .rpc();

  console.log("✅ NFT Purchased! Tx:", tx);
})();