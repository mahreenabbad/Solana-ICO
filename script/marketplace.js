import * as anchor from "@project-serum/anchor";
import {
  Connection,
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  getAssociatedTokenAddress,
  getOrCreateAssociatedTokenAccount,
  createMint,
  TOKEN_PROGRAM_ID,
  getAccount,
  getAssociatedTokenAddressSync,
  createInitializeAccountInstruction,
  createAssociatedTokenAccountInstruction,
  getMinimumBalanceForRentExemptAccount,
  AccountLayout,
} from "@solana/spl-token";
import * as fs from "fs";
import * as path from "path";
import "dotenv/config";
import { fileURLToPath } from "url";
import { Wallet } from "@project-serum/anchor";
import { BN } from "bn.js"; // ⚡ safest — BN is actually from bn.js

// ========= Config =========
const PROGRAM_ID = new PublicKey(
  "76sgJHKdFqnRQ4sheZzjbNFdqFKvS4akdTbeAhsNB3BA"
);
const CLUSTER_URL = "https://api.devnet.solana.com";
//  const CLUSTER_URL = "https://devnet.helius-rpc.com"; // alternative

// Get IDL
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const idlPath = path.join(__dirname, "./marketplace.json");
const idl = JSON.parse(fs.readFileSync(idlPath, "utf-8"));
// import idl from '../target/idl/zktc.json' assert {type : "json"};
if (!idl.version || !idl.instructions || !idl.accounts) {
  throw new Error(
    "❌ IDL is invalid: missing version, instructions, or accounts"
  );
}

console.log("✅ IDL is valid, version:", idl.version);

// Load wallet
// const secretKeyPath = path.join(__dirname, "../new4_program-keypair.json");

const secretKeyPath = path.join(__dirname, "../new_program-keypair.json");
const secretKeyString = fs.readFileSync(secretKeyPath, "utf8");
const secretKey = Uint8Array.from(JSON.parse(secretKeyString));
const payer = Keypair.fromSecretKey(secretKey);
const wallet = new Wallet(payer);

// ========= Provider / Program =========
const connection = new Connection(CLUSTER_URL, "confirmed");
const provider = new anchor.AnchorProvider(connection, wallet, {
  preflightCommitment: "confirmed",
});
anchor.setProvider(provider);
const program = new anchor.Program(idl, PROGRAM_ID, provider);
/////////////

const name = "MyMarketlace"; // <= must be < 33 chars
const fee = 250; // 2.5% (in basis points)

// Derive PDA addresses (must match seeds in your Rust)
const [marketplacePda] = PublicKey.findProgramAddressSync(
  [Buffer.from("marketplace"), Buffer.from(name)],
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
// async function initializeMarketplace() {
//   console.log("Marketplace PDA:", marketplacePda.toBase58());
//   console.log("Rewards Mint PDA:", rewardsMintPda.toBase58());
//   console.log("Treasury PDA:", treasuryPda.toBase58());

//   // Call initialize
//   const tx = await program.methods
//     .initialize(name, fee)
//     .accounts({
//       admin: wallet.publicKey,
//       marketplace: marketplacePda,
//       rewardsMint: rewardsMintPda,
//       treasury: treasuryPda,
//       systemProgram: anchor.web3.SystemProgram.programId,
//       tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
//     })
//     .rpc();

//   console.log("✅ Marketplace initialized!");
//   console.log("🔗 Tx:", tx);
// }

// initializeMarketplace().catch(console.error);

// Marketplace PDA: MrrTd7thuRnLKyKcjckzHKepeY8igGbuECCPCz2ZoGZ
// Rewards Mint PDA: 3cNMH61o6ZXrNdoHxrFz1ZpyW2TcL1Zj7rXmZyn3z1jV
// Treasury PDA: wyM2rycgTQVN8mDNSbdE22pfc4jxbq35hCEiZubc3pV
// ✅ Marketplace initialized!
// 🔗 Tx: 5aeRuz6CbUQNj4ZY1wzGnXHe8YfiTMBJzcCreKjAThY8H5vQmB3fMT8CZd6fAQ5gHK6GCFc5y6pduPDXPV7bwXsg/////////////
/////////////

// Marketplace PDA (already initialized)
// const name = "MyMarket"; // same name you used in initialize()
// const [marketplacePda] = PublicKey.findProgramAddressSync(
//   [Buffer.from("marketplace"), Buffer.from(name)],
//    program.programId
// );

// Listing PDA
const nftMint = new PublicKey("CFSa2tjMU8Qkgt8LChdXRaU4rNGmM1yBs1dhbwzm17Mc"); // the NFT being listed

const makerAta = getAssociatedTokenAddressSync(nftMint, wallet.publicKey);
const [listingPda] = PublicKey.findProgramAddressSync(
  [marketplacePda.toBuffer(), nftMint.toBuffer()],
  program.programId
);
// Vault ATA (belongs to listing PDA as authority)
const vaultAta = getAssociatedTokenAddressSync(
  nftMint,
  listingPda,
  true // allowOwnerOffCurve
);
// Maker’s ATA
// (async () => {
//   const tx = await program.methods
//     .list(new BN(1_000_000_000)) // price in lamports (example: 1 SOL)
//     .accounts({
//       maker: wallet.publicKey,
//       marketplace: marketplacePda,
//       makerMint: nftMint,
//       makerAta: makerAta,
//       vault: vaultAta,
//       listing: listingPda,
//       metadata: (
//         await PublicKey.findProgramAddressSync(
//           [
//             Buffer.from("metadata"),
//             new PublicKey(
//               "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
//             ).toBuffer(),
//             nftMint.toBuffer(),
//           ],
//           new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s") // metadata program
//         )
//       )[0],
//       masterEdition: (
//         await PublicKey.findProgramAddressSync(
//           [
//             Buffer.from("metadata"),
//             new PublicKey(
//               "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
//             ).toBuffer(),
//             nftMint.toBuffer(),
//             Buffer.from("edition"),
//           ],
//           new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s")
//         )
//       )[0],
//       metadataProgram: new PublicKey(
//         "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
//       ),
//       associatedTokenProgram: anchor.web3.ASSOCIATED_TOKEN_PROGRAM_ID,
//       systemProgram: anchor.web3.SystemProgram.programId,
//       tokenProgram: new PublicKey(
//         "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
//       ), // SPL Token program
//     })
//     .signers([payer])
//     .rpc();

//   console.log("✅ NFT Listed, tx:", tx);
// })();
// ✅ NFT Listed, tx: 3Gu1fFTBM9i7uPgHdDEoGjM6B5zFviNRNcnNaUytk9HtP1pQe8FUtVTTFtkrmF2nbEfgMVyUqW8DDnGHfNkBMZH9
// ✅ NFT Listed, tx: 4PWnDoXbWGtzJ4u1BSkmCX4NjAM8hR7FXdLWoisVUVNTtXBhX7URRm8Kaj3bw3MEMQpEnFe16PtBmGenWPJ7PfdU
//////////////

// PDAs

// Send transaction
(async () => {
  const tx = await program.methods
    .delist()
    .accounts({
      maker: wallet.publicKey,
      marketplace: marketplacePda,
      makerMint: nftMint,
      makerAta: makerAta,
      listing: listingPda,
      vault: vaultAta,
      tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();
  console.log("✅ NFT DeListed, tx:", tx);
})();
// ✅ NFT DeListed, tx: 4becpyNsCkDpwtTL4MeE3BkPdu4BUzpeyppRrKeER388DtHgDJhNgT8W6xFUfBkHVBZJWnstphjL5W9xXg7hh8Si

////////////////////////

// ========== Purchase ==========
// const takerAta =new PublicKey("EUcwt15546fposzhRpwqDFNJXpEnqhmhBn64hteKAEpv")
// const vaultAta = new PublicKey("BKvN4i46gXgSxov3raEX8qAycVvGWiZfwg6T9CPujcGZ");
// const treasuryAta = new PublicKey(
//   "8uVRwNDY1zE9We2miD4SUVVeXxePbErCMUpCBpxCK7zF"
// );

// Rewards Mint PDA: 3cNMH61o6ZXrNdoHxrFz1ZpyW2TcL1Zj7rXmZyn3z1jV
// Treasury PDA: wyM2rycgTQVN8mDNSbdE22pfc4jxbq35hCEiZubc3pV
// ✅ Marketplace initialized!
const makerPubkey = new PublicKey(
  "CRb4bz1HSNaGtZX6qxuFrCYfnpND9cDu5ey2REqZuLNE"
); // 👈 seller wallet
const taker = payer; // buyer

// console.log("Taker ATA:", takerAta.address.toBase58());

// const vaultAta = await getAssociatedTokenAddress(
// nftMint,
// listingPda,   // 👈 your program seeds vault with listing PDA as authority
// true          // allow owner off curve
// );

// console.log("Vault ATA (NFT stored during listing):", vaultAta.toBase58());

// const treasuryAta = await getOrCreateAssociatedTokenAccount(
// connection,
// payer,             // payer for creation
// nftMint,        // the SPL token mint used for treasury
// treasuryPda,       // treasury PDA as owner
// true               // allow off curve
// );

// console.log("Treasury ATA:", treasuryAta.address.toBase58());
//     (async () => {
//       // 1. Create/get ATA for taker (buyer)
//       const takerAta = await getOrCreateAssociatedTokenAccount(
//       connection,
//       payer,            // payer for rent if needed
//       nftMint,               // mint of NFT being purchased
//       payer.publicKey,                   // owner of ATA (buyer)
//       true                     // allow owner off curve (for PDA if needed)
//       );
//       const tx = await program.methods
//     .purchase()
//     .accounts({
//       taker: wallet.publicKey,
//       maker: makerPubkey,
//       makerMint:nftMint,
//       marketplace: marketplacePda,
//       takerAta: takerAta.address,
//       vault: vaultAta,
//       rewards: rewardsMintPda,   // still required in your struct
//       listing: listingPda,
//       treasury: treasuryAta,
//       associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
//       systemProgram: anchor.web3.SystemProgram.programId,
//       tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
//     })
//     .signers([taker])
//     .rpc();

//   console.log("✅ NFT Purchased! Tx:", tx);
// })();
