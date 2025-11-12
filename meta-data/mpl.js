import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";
import {  PublicKey } from "@solana/web3.js";


import { percentAmount, generateSigner, signerIdentity, createSignerFromKeypair } from '@metaplex-foundation/umi'
import { TokenStandard, createAndMint, mplTokenMetadata } from '@metaplex-foundation/mpl-token-metadata'
import { createUmi } from '@metaplex-foundation/umi-bundle-defaults';

const umi = createUmi('https://api.devnet.solana.com'); 
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const secretKeyPath = path.join(__dirname, "../new_program-keypair.json");
const secretKeyString = fs.readFileSync(secretKeyPath, "utf8");
const secretKeyArray = JSON.parse(secretKeyString);
const secretKey = new Uint8Array(secretKeyArray);

const userWallet = umi.eddsa.createKeypairFromSecretKey(secretKey);
const userWalletSigner = createSignerFromKeypair(umi, userWallet);


const metadata = {
    name: "Zakat Coin",
     symbol: "ZKT",
      uri: "https://red-electoral-chickadee-62.mypinata.cloud/ipfs/bafkreibgvhzcb54k6ae6f4i4i7ugv6lh7bslmst5efafka272l3tlcap2m", // pinata URI
  
};

umi.use(signerIdentity(userWalletSigner));
umi.use(mplTokenMetadata())
const mint = new PublicKey("Fi7ZwQ3wHDTFDAn5knBrGQ8yMqhw5vgFM9FGFjCADKRn")

createAndMint(umi, {
    mint,
    authority: umi.identity,
    name: metadata.name,
    symbol: metadata.symbol,
    uri: metadata.uri,
    sellerFeeBasisPoints: percentAmount(0),
    decimals: 9,
    amount: 1000000000000,
    tokenOwner: userWallet.publicKey,
    tokenStandard: TokenStandard.Fungible,
}).sendAndConfirm(umi)
    .then(() => {
        console.log("Successfully minted 1 million tokens (", mint.publicKey, ")");
    })
    .catch((err) => {
        console.error("Error minting tokens:", err);
    });
////////
