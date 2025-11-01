#![allow(clippy::result_large_err)]

use {
    anchor_lang::prelude::*,
    anchor_spl::{
        associated_token::AssociatedToken,
        metadata::{
            create_master_edition_v3, create_metadata_accounts_v3,
            mpl_token_metadata::types::DataV2, CreateMasterEditionV3, CreateMetadataAccountsV3,
            Metadata,
        },
        token::{
            mint_to, freeze_account, set_authority, 
            Mint, MintTo, Token, TokenAccount, FreezeAccount, SetAuthority
        },
    },
};

declare_id!("5HapKjJka6MCGrV3eVC4zkotngCJ25bb16yuea7ucgHd");

#[program]
pub mod soul_bound {
    use super::*;

    /// Mint a soulbound NFT that cannot be transferred
    pub fn mint_soulbound_nft(
        ctx: Context<CreateSoulboundToken>,
        nft_name: String,
        nft_symbol: String,
        nft_uri: String,
        recipient: Pubkey, // Add recipient parameter
    ) -> Result<()> {
        msg!("🚀 Minting Soulbound NFT: {} to recipient: {}", nft_name, recipient);

        // Step 1: Mint exactly 1 token to the recipient's ATA
        mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint_account.to_account_info(),
                    to: ctx.accounts.recipient_token_account.to_account_info(),
                    authority: ctx.accounts.payer.to_account_info(),
                },
            ),
            1,
        )?;

        msg!("📝 Creating metadata account");
        // Step 2: Create metadata account
        create_metadata_accounts_v3(
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
            DataV2 {
                name: nft_name.clone(),
                symbol: nft_symbol.clone(),
                uri: nft_uri.clone(),
                seller_fee_basis_points: 0, // No royalties for soulbound NFTs
                creators: None,
                collection: None,
                uses: None,
            },
            false, // Is mutable - set to false for immutable soulbound NFTs
            true,  // Update authority is signer
            None,  // Collection details
        )?;

        msg!("🏆 Creating master edition account");
        // Step 3: Create master edition account  
        create_master_edition_v3(
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
            None, // Max Supply (None = unique NFT)
        )?;

        msg!("🔒 Freezing recipient's token account (making it non-transferable)");
        // Step 4: Freeze the recipient's token account to prevent transfers
        freeze_account(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                FreezeAccount {
                    account: ctx.accounts.recipient_token_account.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    authority: ctx.accounts.payer.to_account_info(),
                },
            ),
        )?;

        msg!("🚫 Revoking mint authority (no more tokens can be minted)");
        // Step 5: Revoke mint authority to prevent additional minting
        set_authority(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                SetAuthority {
                    account_or_mint: ctx.accounts.mint_account.to_account_info(),
                    current_authority: ctx.accounts.payer.to_account_info(),
                },
            ),
            anchor_spl::token::spl_token::instruction::AuthorityType::MintTokens,
            None, // Set to None to revoke
        )?;

        msg!("🔐 Revoking freeze authority (permanent freeze)");
        // Step 6: Revoke freeze authority to make the freeze permanent
        set_authority(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                SetAuthority {
                    account_or_mint: ctx.accounts.mint_account.to_account_info(),
                    current_authority: ctx.accounts.payer.to_account_info(),
                },
            ),
            anchor_spl::token::spl_token::instruction::AuthorityType::FreezeAccount,
            None, // Set to None to revoke
        )?;

        msg!("✅ Soulbound NFT '{}' successfully minted to {} and locked permanently", nft_name, recipient);
        msg!("🏷️  Details: Symbol={}, URI={}", nft_symbol, nft_uri);
        msg!("🪙 Mint Address: {}", ctx.accounts.mint_account.key());

        Ok(())
    }

    /// Original mint function (for regular transferable NFTs) - kept for backward compatibility
    pub fn mint_nft(
        ctx: Context<CreateToken>,
        nft_name: String,
        nft_symbol: String,
        nft_uri: String,
    ) -> Result<()> {
        msg!("Minting Regular NFT (transferable)");
        
        // Same as original implementation
        mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint_account.to_account_info(),
                    to: ctx.accounts.associated_token_account.to_account_info(),
                    authority: ctx.accounts.payer.to_account_info(),
                },
            ),
            1,
        )?;

        msg!("Creating metadata account");
        create_metadata_accounts_v3(
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
            DataV2 {
                name: nft_name,
                symbol: nft_symbol,
                uri: nft_uri,
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            },
            false, // Is mutable
            true,  // Update authority is signer
            None,  // Collection details
        )?;

        msg!("Creating master edition account");
        create_master_edition_v3(
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
            None,
        )?;

        msg!("Regular NFT minted successfully.");
        Ok(())
    }
}
#[derive(Accounts)]
#[instruction(nft_name: String, nft_symbol: String, nft_uri: String, recipient: Pubkey)]
pub struct CreateSoulboundToken<'info> {
    /// The account paying for the transaction
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: This is just the recipient's wallet address used as the authority
    /// for the associated token account. We do not read/write data from this account.
    pub recipient: UncheckedAccount<'info>,

    /// CHECK: Metadata PDA derived inside the program
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint_account.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub metadata_account: UncheckedAccount<'info>,

    /// CHECK: Edition PDA derived inside the program
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint_account.key().as_ref(), b"edition"],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub edition_account: UncheckedAccount<'info>,

    /// Create new mint account for the NFT
    #[account(
        init,
        payer = payer,
        mint::decimals = 0,
        mint::authority = payer.key(),
        mint::freeze_authority = payer.key(),
    )]
    pub mint_account: Account<'info, Mint>,

    /// Associated token account for the recipient
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_account,
        associated_token::authority = recipient,
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}


// Original account context (kept for backward compatibility)
#[derive(Accounts)]
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

    #[account(
        init,
        payer = payer,
        mint::decimals = 0,
        mint::authority = payer.key(),
        mint::freeze_authority = payer.key(),
    )]
    pub mint_account: Account<'info, Mint>,

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

// Program Id: 5HapKjJka6MCGrV3eVC4zkotngCJ25bb16yuea7ucgHd

// Signature: 3WuGpzVoE1VNHt7poR9V4k5CzrMHpjZMLkrQUoxDMZL5KW2ou9uzYJTUxSUu8VNXKUAUE1aVQM7FHwnGDwjjpsUf
