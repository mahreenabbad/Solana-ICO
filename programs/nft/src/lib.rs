#![allow(clippy::result_large_err)]//Compiler directive: tells Rust/Clippy to ignore warnings about large Result error types.

use {
    anchor_lang::prelude::*,//Anchor framework essentials (macros, Context, Result, Accounts, Signer, etc.).
    anchor_spl::{ //Anchor wrappers for SPL programs:
        associated_token::AssociatedToken, //used to create Associated Token Accounts (ATA).
        metadata::{//helper functions & types for interacting with Metaplex Token Metadata
            create_master_edition_v3, create_metadata_accounts_v3,
            mpl_token_metadata::types::DataV2, CreateMasterEditionV3, CreateMetadataAccountsV3,
            Metadata,
        },
        token::{mint_to, Mint, MintTo, Token, TokenAccount},
    },
};

declare_id!("5eMDSRzaq9NUvurN2s5Q2zmCs8cFPaCA2uhz89kSS6pf");

#[program]//tells Anchor: "these functions are callable instructions".
pub mod nft { //Marks this as the Anchor program module.
    use super::*; //brings everything from the outer scope (the parent module) into this module.

    pub fn mint_nft(
        ctx: Context<CreateToken>,
        nft_name: String,
        nft_symbol: String,
        nft_uri: String,
    ) -> Result<()> {
        msg!("Minting Token"); //Prints log on-chain for debugging/tracing.
        // Cross Program Invocation (CPI)
        // Invoking the mint_to instruction on the token program
        mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint_account.to_account_info(),//NFT mint account.
                    to: ctx.accounts.associated_token_account.to_account_info(),//to: user’s ATA (NFT holder).
                    authority: ctx.accounts.payer.to_account_info(),//authority: payer (who has minting authority).
                },
            ),
            1,//mints 1 token to user’s associated token account (ATA).
        )?;

        msg!("Creating metadata account");
        // Cross Program Invocation (CPI)
        // Invoking the create_metadata_account_v3 instruction on the token metadata program
        create_metadata_accounts_v3( //Creates Metaplex Metadata Account for the NFT:
            CpiContext::new(
                ctx.accounts.token_metadata_program.to_account_info(),
                CreateMetadataAccountsV3 {
                    metadata: ctx.accounts.metadata_account.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    mint_authority: ctx.accounts.payer.to_account_info(),
                    update_authority: ctx.accounts.payer.to_account_info(),
                    payer: ctx.accounts.payer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
            ),
            DataV2 { //Uses DataV2 struct for NFT metadata.
                name: nft_name,
                symbol: nft_symbol,
                uri: nft_uri,
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            },
            false, // Is immutable
            true,  // Update authority is signer
            None,  // Collection details
        )?;

        msg!("Creating master edition account");
        // Cross Program Invocation (CPI)
        // Invoking the create_master_edition_v3 instruction on the token metadata program
        create_master_edition_v3( //Ties metadata to mint.
            CpiContext::new(
                ctx.accounts.token_metadata_program.to_account_info(),
                CreateMasterEditionV3 {
                    edition: ctx.accounts.edition_account.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    update_authority: ctx.accounts.payer.to_account_info(),
                    mint_authority: ctx.accounts.payer.to_account_info(),
                    payer: ctx.accounts.payer.to_account_info(),
                    metadata: ctx.accounts.metadata_account.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
            ),
            None, // Max Supply
        )?;

        msg!("NFT minted successfully.");

        Ok(())
    }
}

#[derive(Accounts)] //Defines all accounts required for mint_nft.
pub struct CreateToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Validate address by deriving pda
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint_account.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub metadata_account: UncheckedAccount<'info>,

    /// CHECK: Validate address by deriving pda
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint_account.key().as_ref(), b"edition"],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub edition_account: UncheckedAccount<'info>,

    // Create new mint account, NFTs have 0 decimals
    #[account(
        init,
        payer = payer,
        mint::decimals = 0,
        mint::authority = payer.key(),
        mint::freeze_authority = payer.key(),
    )]
    pub mint_account: Account<'info, Mint>,

    // Create associated token account, if needed
    // This is the account that will hold the NFT
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_account,
        associated_token::authority = payer,
    )]
    pub associated_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// Program Id: 5eMDSRzaq9NUvurN2s5Q2zmCs8cFPaCA2uhz89kSS6pf

// Signature: 1pmdw7xCij1iHUz5cYX3L1ebAv3bAX6wPAAEnUndMZVNxJNQtvme9UfhT3b8ykYJf61pUZkhG1RGLwmJNfjWMaM