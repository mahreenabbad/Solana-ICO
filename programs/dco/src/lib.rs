use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use std::convert::TryFrom;

declare_id!("CEHTQCjD4A4z6MYRjXydYvFhnz6s5E5Wha8XvH9xvFXM");
#[program]
pub mod dco {
    use super::*;
    

    /// Initialize DCO: creates `state` PDA and `vault` token account (authority = state PDA).
    /// - `token_price`: u128 on-chain price (client must pass BN)
    /// - `dco_end_time`: i64 unix timestamp (client must pass BN)
    /// - `zcw`: Pubkey of charity/token-account owner (used later in donate_to_zcw)
    pub fn initialize(
        ctx: Context<Initialize>,
        token_price: u128,
        dco_end_time: i64,
        zcw: Pubkey,
    ) -> Result<()> {
        require!(token_price > 0, DcoError::TokenPriceMustBeGreaterThanZero);
        require!(
            dco_end_time > Clock::get()?.unix_timestamp,
            DcoError::DcoNotActive
        );

        let state = &mut ctx.accounts.state;

        // set core fields
        state.owner = ctx.accounts.owner.key();
        state.zk_token_mint = ctx.accounts.token_mint.key();
        state.vault = ctx.accounts.vault.key();
        state.token_price = token_price;
        state.token_sold = 0u128;
        state.total_donations = 0u128;
        state.dco_end_time = dco_end_time;
        state.zcw = zcw;
        state.releasers = Vec::new();

        // access bump directly from ctx.bumps struct
        state.bump = ctx.bumps.state;

        emit!(Initialized {
            owner: ctx.accounts.owner.key(),
            zk_token_mint: ctx.accounts.token_mint.key(),
            vault: ctx.accounts.vault.key(),
            token_price,
            dco_end_time,
            zcw,
        });

        Ok(())
    }
///Anyone  “try” to call it, but their transaction will fail because they cannot sign as the owner.
    /// Owner transfers existing zktc tokens from `from` (owner's token account) into the vault.
    pub fn inject_supply(ctx: Context<InjectSupply>, amount: u64) -> Result<()> {
        require!(amount > 0, DcoError::AmountMustBeGreaterThanZero);

        // transfer from owner -> vault (owner signs)
        let cpi_accounts = Transfer {
            from: ctx.accounts.from.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        emit!(SupplyInjected {
            owner: ctx.accounts.owner.key(),
            amount,
        });
        Ok(())
    }

    /// Release tokens from vault to buyer_token_account (PDA signs). Caller must be owner or a releaser.
    /// donation_amount is accounted to `total_donations` but not immediately transferred out of the vault.
   pub fn release_zktc(ctx: Context<ReleaseZktc>, amount: u64, donation_amount: u64) -> Result<()> {
    require!(amount > 0, DcoError::AmountMustBeGreaterThanZero);
    require!(donation_amount > 0, DcoError::AmountMustBeGreaterThanZero);

    // Check if vault has sufficient tokens
    require!(
        amount <= ctx.accounts.vault.amount,
        DcoError::InsufficientTokens
    );

    // Caller authorization
    let state_key = ctx.accounts.state.owner;
    require!(
        ctx.accounts.caller.key() == state_key
            || ctx.accounts.state.releasers.contains(&ctx.accounts.caller.key()),
        DcoError::Unauthorized
    );

    // build PDA signer seeds
    let bump = ctx.accounts.state.bump;
    let seeds = &[
        b"dco_state".as_ref(),
        state_key.as_ref(),
        &[bump],
    ];
    let signer = &[&seeds[..]];

    // --- do the transfer in a scope with only immutable borrow ---
    {
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: ctx.accounts.state.to_account_info(), // immutable borrow
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, amount)?;
    }

    // --- now borrow mutably AFTER CPI ---
    let state = &mut ctx.accounts.state;

    state.token_sold = state
        .token_sold
        .checked_add(amount as u128)
        .ok_or(DcoError::MathOverflow)?;

    state.total_donations = state
        .total_donations
        .checked_add(donation_amount as u128)
        .ok_or(DcoError::MathOverflow)?;

    emit!(ZktcReleased {
        caller: ctx.accounts.caller.key(),
        buyer: ctx.accounts.buyer_token_account.key(),
        amount,
        donation_amount,
    });

    Ok(())
}

    /// Transfer the accumulated donations from vault -> zcw_token_account (signed by PDA).
    /// After success, reset total_donations to 0.
    pub fn donate_to_zcw(ctx: Context<DonateToZcw>) -> Result<()> {
        // authorization: owner or releaser
        require!(
            ctx.accounts.caller.key() == ctx.accounts.state.owner
                || ctx.accounts.state.releasers.contains(&ctx.accounts.caller.key()),
            DcoError::Unauthorized
        );

        require!(ctx.accounts.state.total_donations > 0, DcoError::NoDonationsMade);

        // Get values we need before the transfer
        let donation_amount = ctx.accounts.state.total_donations;
        let owner_key = ctx.accounts.state.owner;
        let bump = ctx.accounts.state.bump;
        let caller_key = ctx.accounts.caller.key();
        let zcw_key = ctx.accounts.zcw_token_account.key();

        // convert u128 -> u64 for SPL transfer
        let transfer_amount = u64::try_from(donation_amount).map_err(|_| DcoError::AmountTooLarge)?;

        // Check if vault has sufficient tokens for donation
        require!(
            transfer_amount <= ctx.accounts.vault.amount,
            DcoError::InsufficientTokens
        );

        // build signer using same PDA seeds
        let seeds = &[
            b"dco_state".as_ref(),
            owner_key.as_ref(),
            &[bump],
        ];
        let signer = &[&seeds[..]];

        // Perform transfer in a scope to end the immutable borrow
        {
            let cpi_accounts = Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.zcw_token_account.to_account_info(),
                authority: ctx.accounts.state.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
            token::transfer(cpi_ctx, transfer_amount)?;
        }

        // Now we can mutably borrow state to reset donations
        let state = &mut ctx.accounts.state;
        state.total_donations = 0u128;

        emit!(DonationSent {
            caller: caller_key,
            zcw: zcw_key,
            amount: transfer_amount,
        });

        Ok(())
    }

    /// Owner withdraws remaining tokens after DCO end time (signed by PDA).
    pub fn withdraw(ctx: Context<OnlyOwnerAfterEnd>, amount: u64) -> Result<()> {
        require!(amount > 0, DcoError::AmountMustBeGreaterThanZero);
        require!(
            amount <= ctx.accounts.vault.amount,
            DcoError::InsufficientTokens
        );

        let state = &ctx.accounts.state;
        let bump = state.bump;
        let seeds = &[
            b"dco_state".as_ref(),
            state.owner.as_ref(),
            &[bump],
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.owner_token_account.to_account_info(),
            authority: ctx.accounts.state.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, amount)?;

        emit!(Withdrawn {
            owner: ctx.accounts.owner.key(),
            amount,
        });

        Ok(())
    }

    /// Add a new releaser (owner-only)
    pub fn add_releaser(ctx: Context<OnlyOwner>, new_releaser: Pubkey) -> Result<()> {
        let state = &mut ctx.accounts.state;

        require!(
            state.releasers.len() < DcoState::MAX_RELEASERS,
            DcoError::ReleasersFull
        );

        if !state.releasers.contains(&new_releaser) {
            state.releasers.push(new_releaser);
        }

        emit!(ReleaserAdded {
            owner: ctx.accounts.owner.key(),
            new_releaser,
        });
        Ok(())
    }

    /// Remove a releaser (owner-only)
    pub fn remove_releaser(ctx: Context<OnlyOwner>, old_releaser: Pubkey) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.releasers.retain(|r| *r != old_releaser);

        emit!(ReleaserRemoved {
            owner: ctx.accounts.owner.key(),
            old_releaser,
        });
        Ok(())
    }

}

//
// --- Accounts / State types ---
//

#[derive(Accounts)]
#[instruction(token_price: u128, dco_end_time: i64, zcw: Pubkey)]
pub struct Initialize<'info> {
    /// Owner who creates the DCO and pays for account creation
    #[account(mut)]
    pub owner: Signer<'info>,

    /// The zktc token mint (must be an existing Mint)
    pub token_mint: Account<'info, Mint>,

    /// DCO state PDA: seeds = [b"dco_state", owner.key().as_ref()]
    #[account(
        init,
        payer = owner,
        space = DcoState::SPACE,
        seeds = [b"dco_state", owner.key().as_ref()],
        bump
    )]
    pub state: Account<'info, DcoState>,

    /// Vault token account for holding zktc tokens.
    /// seeds = [b"dco_vault", state.key().as_ref()]
    /// token::authority = state (PDA)
    #[account(
        init,
        payer = owner,
        token::mint = token_mint,
        token::authority = state,
        seeds = [b"dco_vault", state.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InjectSupply<'info> {
    #[account(mut, has_one = owner)]
    pub state: Account<'info, DcoState>,

    #[account(
        mut,
        constraint = from.mint == state.zk_token_mint @ DcoError::InvalidTokenMint,
        constraint = from.owner == owner.key() @ DcoError::Unauthorized
    )]
    pub from: Account<'info, TokenAccount>,

    #[account(mut, constraint = vault.mint == state.zk_token_mint @ DcoError::InvalidTokenMint)]
    pub vault: Account<'info, TokenAccount>,

    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ReleaseZktc<'info> {
    #[account(mut, has_one = vault)]
    pub state: Account<'info, DcoState>,

    /// Caller (owner or releaser)
    pub caller: Signer<'info>,

    /// Vault token account (PDA authority)
    #[account(mut, constraint = vault.mint == state.zk_token_mint @ DcoError::InvalidTokenMint)]
    pub vault: Account<'info, TokenAccount>,

    /// Buyer's token account (must be token account for same mint)
    #[account(
        mut,
        constraint = buyer_token_account.mint == state.zk_token_mint @ DcoError::InvalidTokenMint
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct DonateToZcw<'info> {
    #[account(mut, has_one = vault)]
    pub state: Account<'info, DcoState>,

    pub caller: Signer<'info>,

    #[account(mut, constraint = vault.mint == state.zk_token_mint @ DcoError::InvalidTokenMint)]
    pub vault: Account<'info, TokenAccount>,

    /// zcw token account (destination)
    #[account(
        mut,
        constraint = zcw_token_account.mint == state.zk_token_mint @ DcoError::InvalidTokenMint
    )]
    pub zcw_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct OnlyOwner<'info> {
    #[account(mut, has_one = owner)]
    pub state: Account<'info, DcoState>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct OnlyOwnerAfterEnd<'info> {
    #[account(
        mut,
        has_one = owner,
        constraint = Clock::get()?.unix_timestamp >= state.dco_end_time @ DcoError::DcoNotActive
    )]
    pub state: Account<'info, DcoState>,

    pub owner: Signer<'info>,

    #[account(mut, constraint = vault.mint == state.zk_token_mint @ DcoError::InvalidTokenMint)]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = owner_token_account.mint == state.zk_token_mint @ DcoError::InvalidTokenMint,
        constraint = owner_token_account.owner == owner.key() @ DcoError::Unauthorized
    )]
    pub owner_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

/// On-chain state
#[account]
pub struct DcoState {
    pub owner: Pubkey,
    pub zk_token_mint: Pubkey,
    pub vault: Pubkey,
    pub token_price: u128,
    pub token_sold: u128,
    pub total_donations: u128,
    pub dco_end_time: i64,
    pub zcw: Pubkey,
    pub releasers: Vec<Pubkey>,
    pub bump: u8,
}

impl DcoState {
    pub const MAX_RELEASERS: usize = 20;
    // computed size for the fields above
    pub const SPACE: usize = 8 + // discriminator
        32 + // owner
        32 + // zk_token_mint
        32 + // vault
        16 + // token_price (u128)
        16 + // token_sold (u128)
        16 + // total_donations (u128)
        8  + // dco_end_time (i64)
        32 + // zcw
        4  + (32 * Self::MAX_RELEASERS) + // releasers vec (len + entries)
        1;   // bump
}

#[error_code]
pub enum DcoError {
    #[msg("You are not Authorized")]
    Unauthorized,
    #[msg("Token price must be greater than zero")]
    TokenPriceMustBeGreaterThanZero,
    #[msg("Amount must be greater than zero")]
    AmountMustBeGreaterThanZero,
    #[msg("No tokens remaining")]
    NoTokensRemaining,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("No balance to withdraw")]
    NoBalanceToWithdraw,
    #[msg("Transfer failed")]
    TransferFailed,
    #[msg("Insufficient tokens")]
    InsufficientTokens,
    #[msg("Transaction already processed")]
    TransactionAlreadyProcessed,
    #[msg("Global release time not reached")]
    GlobalReleaseTimeNotReached,
    #[msg("Cannot withdraw before global release")]
    CannotWithdrawBeforeGlobalRelease,
    #[msg("DCO is not active")]
    DcoNotActive,
    #[msg("Invalid address")]
    InvalidAddress,
    #[msg("No donations made")]
    NoDonationsMade,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Amount too large to fit into u64")]
    AmountTooLarge,
    #[msg("Releasers list full")]
    ReleasersFull,
    #[msg("Invalid token mint")]
    InvalidTokenMint,
}

//
// Events
//
#[event]
pub struct Initialized {
    pub owner: Pubkey,
    pub zk_token_mint: Pubkey,
    pub vault: Pubkey,
    pub token_price: u128,
    pub dco_end_time: i64,
    pub zcw: Pubkey,
}

#[event]
pub struct SupplyInjected {
    pub owner: Pubkey,
    pub amount: u64,
}

#[event]
pub struct ZktcReleased {
    pub caller: Pubkey,
    pub buyer: Pubkey,
    pub amount: u64,
    pub donation_amount: u64,
}

#[event]
pub struct DonationSent {
    pub caller: Pubkey,
    pub zcw: Pubkey,
    pub amount: u64,
}

#[event]
pub struct Withdrawn {
    pub owner: Pubkey,
    pub amount: u64,
}

#[event]
pub struct ReleaserAdded {
    pub owner: Pubkey,
    pub new_releaser: Pubkey,
}

#[event]
pub struct ReleaserRemoved {
    pub owner: Pubkey,
    pub old_releaser: Pubkey,
}

// add setter function for zcw and dco_end_time
// Program Id: CEHTQCjD4A4z6MYRjXydYvFhnz6s5E5Wha8XvH9xvFXM

// Signature: 2wADrwjyH2UhFiWV7DRjLVPf1StZEoSeWoQ1rk46Y9UYUP4J9zMkLAVN3x7Tw6SRWi5PK9RoLTSoc92N3fPq2oPi
