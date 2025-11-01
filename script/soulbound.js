import * as anchor from "@project-serum/anchor";
import { Connection, PublicKey, Keypair } from "@solana/web3.js";
import { getAssociatedTokenAddress,
  TOKEN_PROGRAM_ID} from "@solana/spl-token";

import * as fs from "fs";
import * as path from "path";
import "dotenv/config";
import { fileURLToPath } from "url";
import { Wallet } from "@project-serum/anchor";

const TOKEN_METADATA_PROGRAM_ID = new PublicKey(
  "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
);

// ========= Config =========
const PROGRAM_ID = new PublicKey("5HapKjJka6MCGrV3eVC4zkotngCJ25bb16yuea7ucgHd");
const CLUSTER_URL = "https://api.devnet.solana.com";

// Get IDL - try multiple methods
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const idlPath = path.join(__dirname,"./soulbound.json");

let idl;
try {
  // First try to read local IDL file
  idl = JSON.parse(fs.readFileSync(idlPath, "utf-8"));
  console.log("✅ Using local IDL file, version:", idl.version);
} catch (error) {
  console.log("❌ Could not read local IDL file:", error.message);
  throw error;
}

// Verify IDL has correct program ID
if (idl.metadata && idl.metadata.address !== PROGRAM_ID.toString()) {
  console.log(`⚠️  IDL program ID mismatch:`);
  console.log(`   IDL has: ${idl.metadata.address}`);
  console.log(`   Expected: ${PROGRAM_ID.toString()}`);
  console.log(`   Fixing IDL program ID...`);
  idl.metadata.address = PROGRAM_ID.toString();
}

if (!idl.version || !idl.instructions) {
  throw new Error("❌ IDL is invalid: missing version or instructions");
}

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

// Helper functions
// Replace with the actual recipient of NFT
const recipient = new PublicKey("AWxG813cbExDm9fFKq7ofFWtn2inENpuiymvPaUzaHjq");

// Generate new mint account
const mintKeypair = Keypair.generate();
    console.log("Mint Address:", mintKeypair.publicKey.toBase58());


const [metadataPDA] = PublicKey.findProgramAddressSync(
  [
    Buffer.from("metadata"),
    TOKEN_METADATA_PROGRAM_ID.toBuffer(),
    mintKeypair.publicKey.toBuffer(),
  ],
  TOKEN_METADATA_PROGRAM_ID
);

const [editionPDA] = PublicKey.findProgramAddressSync(
  [
    Buffer.from("metadata"),
    TOKEN_METADATA_PROGRAM_ID.toBuffer(),
    mintKeypair.publicKey.toBuffer(),
    Buffer.from("edition"),
  ],
  TOKEN_METADATA_PROGRAM_ID
);

async function mintSoulboundNFT() {
 
  const nftName = "My Soulbound NFT";
  const nftSymbol = "SBT";
  const nftUri = "https://red-electoral-chickadee-62.mypinata.cloud/ipfs/bafkreiarg6hr4j2p3aaxpaqxyldh5anokfityfaqx5exgag3ja56jnb4cy"; // Metadata JSON link
  const ata = await getAssociatedTokenAddress(mintKeypair.publicKey, recipient);

  try {
    const tx = await program.methods
      .mintSoulboundNft(nftName, nftSymbol, nftUri, recipient)
      .accounts({
        payer: payer.publicKey,
        recipient: recipient,
        metadataAccount: metadataPDA,
        editionAccount: editionPDA,
        mintAccount: mintKeypair.publicKey,
        recipientTokenAccount: ata,
        tokenProgram: TOKEN_PROGRAM_ID,
        tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
        associatedTokenProgram:  new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"),
        systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
        .signers([mintKeypair]) 
      .rpc();

    console.log("✅ Soulbound NFT Minted. Tx Signature:", tx);
  } catch (err) {
    console.error("❌ Error minting NFT:", err);
  }
}

mintSoulboundNFT();
