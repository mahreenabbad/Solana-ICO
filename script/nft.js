

import * as anchor from "@project-serum/anchor";
import { Connection, PublicKey, Keypair } from "@solana/web3.js";
import { getAssociatedTokenAddress,getOrCreateAssociatedTokenAccount,createMint ,TOKEN_PROGRAM_ID} from "@solana/spl-token";
import * as fs from "fs";
import * as path from "path";
import "dotenv/config";
import { fileURLToPath } from "url";
import {  Wallet } from "@project-serum/anchor";
import { BN } from "bn.js"; // ⚡ safest — BN is actually from bn.js
// import { TOKEN_PROGRAM_ID, getAssociatedTokenAddress } from "@solana/spl-token";

// ========= Config =========
const PROGRAM_ID = new PublicKey("5eMDSRzaq9NUvurN2s5Q2zmCs8cFPaCA2uhz89kSS6pf");
const CLUSTER_URL = "https://api.devnet.solana.com";
// const CLUSTER_URL = "https://solana-devnet.rpcpool.com"; // alternative

// Get IDL
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const idlPath = path.join(__dirname, "./nft.json");
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

/////////

async function getMetadataAndEdition(mint, tokenMetadataProgramId) {
  const [metadata] = await PublicKey.findProgramAddress(
    [
      Buffer.from("metadata"),
      tokenMetadataProgramId.toBuffer(),
      mint.toBuffer(),
    ],
    tokenMetadataProgramId
  );

  const [edition] = await PublicKey.findProgramAddress(
    [
      Buffer.from("metadata"),
      tokenMetadataProgramId.toBuffer(),
      mint.toBuffer(),
      Buffer.from("edition"),
    ],
    tokenMetadataProgramId
  );

  return { metadata, edition };
}


async function mintNft() {
  // Create mint account
  const mint = Keypair.generate();

  const metadataProgramId = new PublicKey(
    "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s" // Metaplex Token Metadata Program ID
  );

  const { metadata, edition } = await getMetadataAndEdition(mint.publicKey, metadataProgramId);

  // Derive ATA for payer
  const ata = await getAssociatedTokenAddress(mint.publicKey, payer.publicKey);

  // Call your program
const tx = await program.methods
    .mintNft("MyNFT", "MNFT", "https://red-electoral-chickadee-62.mypinata.cloud/ipfs/bafkreiarg6hr4j2p3aaxpaqxyldh5anokfityfaqx5exgag3ja56jnb4cy")
    .accounts({
      payer: payer.publicKey,
      metadataAccount: metadata,
      editionAccount: edition,
      mintAccount: mint.publicKey,
      associatedTokenAccount: ata,
      tokenProgram: TOKEN_PROGRAM_ID,
      tokenMetadataProgram: metadataProgramId,
      associatedTokenProgram: new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"),
      systemProgram: anchor.web3.SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .signers([payer, mint])//Since your mint_account is a new Keypair (not a PDA), the runtime needs to know:
// You’re authorized to create it.
// That’s proven by signing the transaction with the mint keypair.
    .rpc();

  console.log("✅ NFT minted:", mint.publicKey.toBase58());
      console.log("🔗 Transaction:", tx);
}

mintNft()
// ✅ NFT minted: CFSa2tjMU8Qkgt8LChdXRaU4rNGmM1yBs1dhbwzm17Mc
// 🔗 Transaction: 2my5wESJzKkfXo9gZtrFk1kacVni2F6orWBJWdqp5WHWisJSWsvWiuvEvMdQLX4JSavFCE93ULHdSutLfucUrD5y

////////
//Non transferable nft
// import * as anchor from "@project-serum/anchor";
// import {
//   Connection,
//   PublicKey,
//   Keypair,
//   SystemProgram,
// } from "@solana/web3.js";
// import {
//   getAssociatedTokenAddress,
//   ExtensionType,
//   getMintLen,
//   createInitializeMintInstruction,
//   createInitializeNonTransferableMintInstruction,
//   TOKEN_2022_PROGRAM_ID,
// } from "@solana/spl-token";
// import * as fs from "fs";
// import * as path from "path";
// import "dotenv/config";
// import { fileURLToPath } from "url";
// import { Wallet } from "@project-serum/anchor";

// // ========= Config =========
// const PROGRAM_ID = new PublicKey("5eMDSRzaq9NUvurN2s5Q2zmCs8cFPaCA2uhz89kSS6pf");
// const CLUSTER_URL = "https://api.devnet.solana.com";

// // ========= Load IDL =========
// const __filename = fileURLToPath(import.meta.url);
// const __dirname = path.dirname(__filename);
// const idlPath = path.join(__dirname, "./nft.json");
// const idl = JSON.parse(fs.readFileSync(idlPath, "utf-8"));
// if (!idl.version || !idl.instructions || !idl.accounts) {
//   throw new Error("❌ IDL is invalid: missing version, instructions, or accounts");
// }
// console.log("✅ IDL is valid, version:", idl.version);

// // ========= Wallet =========
// const secretKeyPath = path.join(__dirname, "../new_program-keypair.json");
// const secretKeyString = fs.readFileSync(secretKeyPath, "utf8");
// const secretKey = Uint8Array.from(JSON.parse(secretKeyString));
// const payer = Keypair.fromSecretKey(secretKey);
// const wallet = new Wallet(payer);

// // ========= Provider / Program =========
// const connection = new Connection(CLUSTER_URL, "confirmed");
// const provider = new anchor.AnchorProvider(connection, wallet, {
//   preflightCommitment: "confirmed",
// });
// anchor.setProvider(provider);
// const program = new anchor.Program(idl, PROGRAM_ID, provider);

// // ========= Utils =========
// async function getMetadataAndEdition(mint, tokenMetadataProgramId) {
//   const [metadata] = await PublicKey.findProgramAddress(
//     [Buffer.from("metadata"), tokenMetadataProgramId.toBuffer(), mint.toBuffer()],
//     tokenMetadataProgramId
//   );
//   const [edition] = await PublicKey.findProgramAddress(
//     [Buffer.from("metadata"), tokenMetadataProgramId.toBuffer(), mint.toBuffer(), Buffer.from("edition")],
//     tokenMetadataProgramId
//   );
//   return { metadata, edition };
// }

// // ========= Mint Soulbound NFT =========
// async function mintNft() {
//   // 🆕 Generate new mint
//   const mint = Keypair.generate();

//   // Setup Metadata Program (Metaplex)
//   const metadataProgramId = new PublicKey(
//     "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
//   );
//   const { metadata, edition } = await getMetadataAndEdition(
//     mint.publicKey,
//     metadataProgramId
//   );

//   // Get rent-exempt space for a non-transferable mint
//   const extensions = [ExtensionType.NonTransferable];
//   const mintLen = getMintLen(extensions);
//   const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);

//   // Create ATA
//   const ata = await getAssociatedTokenAddress(
//     mint.publicKey,
//     payer.publicKey,
//     false,
//     TOKEN_2022_PROGRAM_ID
//   );

//   // Build transaction
//   const tx = new anchor.web3.Transaction().add(
//     // Create the mint account with Token-2022 program
//     SystemProgram.createAccount({
//       fromPubkey: payer.publicKey,
//       newAccountPubkey: mint.publicKey,
//       space: mintLen,
//       lamports,
//       programId: TOKEN_2022_PROGRAM_ID,
//     }),
//     // Initialize NonTransferable extension
//     createInitializeNonTransferableMintInstruction(
//       mint.publicKey,
//       TOKEN_2022_PROGRAM_ID
//     ),
//     // Initialize the mint itself
//     createInitializeMintInstruction(
//       mint.publicKey,
//       0, // decimals (0 for NFT)
//       payer.publicKey,
//       null,
//       TOKEN_2022_PROGRAM_ID
//     )
//   );

//   // Send tx to create mint
//   await anchor.web3.sendAndConfirmTransaction(connection, tx, [payer, mint]);

//   // Call your Anchor program to mint + metadata
//   const sig = await program.methods
//     .mintNft(
//       "MySoulboundNFT",
//       "SBNFT",
//       "https://red-electoral-chickadee-62.mypinata.cloud/ipfs/bafkreiarg6hr4j2p3aaxpaqxyldh5anokfityfaqx5exgag3ja56jnb4cy"
//     )
//     .accounts({
//       payer: payer.publicKey,
//       metadataAccount: metadata,
//       editionAccount: edition,
//       mintAccount: mint.publicKey,
//       associatedTokenAccount: ata,
//       tokenProgram: TOKEN_2022_PROGRAM_ID,
//       tokenMetadataProgram: metadataProgramId,
//       associatedTokenProgram: new PublicKey(
//         "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
//       ),
//       systemProgram: anchor.web3.SystemProgram.programId,
//       rent: anchor.web3.SYSVAR_RENT_PUBKEY,
//     })
//     .signers([payer, mint])
//     .rpc();

//   console.log("✅ Soulbound NFT Minted:", mint.publicKey.toBase58());
//   console.log("🔗 Tx:", sig);
// }

// mintNft();
