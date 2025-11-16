use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, MintTo, Transfer};
use anchor_spl::metadata::{create_metadata_accounts_v3, CreateMetadataAccountsV3, Metadata};
use mpl_token_metadata::types::DataV2;


use std::convert::TryFrom;

declare_id!("2914jUPpQ9F6bVuktZH7BLpEY5rjeJjmU42WsRafoeqV");

#[program]
pub mod engines {
    use super::*;

    /// Initialize the Engines program
    pub fn initialize(
        ctx: Context<Initialize>,
        zcw: Pubkey,
        uri_30_days: String,
        uri_60_days: String,
        uri_180_days: String,
        uri_365_days: String,
    ) -> Result<()> {
        let state = &mut ctx.accounts.state;

        state.authority = ctx.accounts.authority.key();
        state.zktc_mint = ctx.accounts.zktc_mint.key();
        state.zcw = zcw;
        state.vault = ctx.accounts.vault.key();
        state.total_locked = 0;
        state.total_unlocked = 0;
        state.next_badge_id = 1;
        state.next_lock_id = 1;
        state.reserve_for_donation = 0;
        state.paused = false;
        state.bump = ctx.bumps.state;

        // Set default donation rates (in basis points)
        state.donation_rates = [50, 100, 150, 250]; // 0.5%, 1%, 1.5%, 2.5%
        state.scale = 10000;

        // Set period durations in seconds (for testing: minutes instead of days)
        state.period_durations = [60, 120, 180, 240]; // 1, 2, 3, 4 minutes

        // set URIs
        state.uri_30_days = uri_30_days;
        state.uri_60_days = uri_60_days;
        state.uri_180_days = uri_180_days;
        state.uri_365_days = uri_365_days;

        emit!(Initialized {
            authority: ctx.accounts.authority.key(),
            zktc_mint: ctx.accounts.zktc_mint.key(),
            zcw,
        });

        Ok(())
    }

    /// Lock tokens for a specific period
    pub fn lock_tokens(ctx: Context<LockTokens>, amount: u64, period: Period) -> Result<()> {
        require!(amount > 0, EnginesError::InvalidAmount);
        require!(!ctx.accounts.state.paused, EnginesError::Paused);

        let state = &mut ctx.accounts.state;
        let clock = Clock::get()?;

        if matches!(period, Period::D180 | Period::D355) {
            let rate = state.donation_rates[period as usize];
            let donation = (amount as u128 * rate as u128) / state.scale as u128;
            let donation_u64 = u64::try_from(donation).map_err(|_| EnginesError::AmountTooLarge)?;

            let available = available_excess(
                ctx.accounts.vault.amount,
                state.total_locked,
                state.reserve_for_donation,
            );
            require!(
                available >= donation_u64,
                EnginesError::InsufficientMatchingTreasury
            );

            state.reserve_for_donation = state
                .reserve_for_donation
                .checked_add(donation_u64)
                .ok_or(EnginesError::MathOverflow)?;
        }

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        let lock_id = state.next_lock_id;
        state.next_lock_id = state
            .next_lock_id
            .checked_add(1)
            .ok_or(EnginesError::MathOverflow)?;

        let lock_position = LockPosition {
            id: lock_id,
            amount,
            start: clock.unix_timestamp as u64,
            period,
            unlocked: false,
        };

        let user_locks = &mut ctx.accounts.user_data;
        require!(
            user_locks.locks.len() < MAX_LOCKS_PER_USER,
            EnginesError::TooManyLocks
        );
        user_locks.locks.push(lock_position);
        user_locks.owner = ctx.accounts.user.key();

        state.total_locked = state
            .total_locked
            .checked_add(amount)
            .ok_or(EnginesError::MathOverflow)?;

        emit!(TokensLocked {
            user: ctx.accounts.user.key(),
            lock_id,
            amount,
            period,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Unlock a specific lock by ID
    pub fn unlock_tokens(ctx: Context<UnlockTokens>, lock_id: u64) -> Result<()> {
        require!(!ctx.accounts.state.paused, EnginesError::Paused);
        let mut ctx = ctx;

        let user_locks = &mut ctx.accounts.user_data;

        let mut found_lock = None;
        for (index, lock) in user_locks.locks.iter().enumerate() {
            if lock.id == lock_id && !lock.unlocked {
                found_lock = Some(index);
                break;
            }
        }

        let lock_index = found_lock.ok_or(EnginesError::BadLockId)?;
        unlock_by_index(&mut ctx, lock_index)?;

        Ok(())
    }

    /// Unlock all matured locks for a user
    pub fn unlock_all_matured(ctx: Context<UnlockTokens>) -> Result<()> {
        require!(!ctx.accounts.state.paused, EnginesError::Paused);
        let mut ctx = ctx;

        let user_locks = &mut ctx.accounts.user_data;
        let clock = Clock::get()?;
        let durations = ctx.accounts.state.period_durations;
        let mut unlocked_count = 0u32;
        let mut indices_to_unlock: Vec<usize> = Vec::new();

        for (index, lock) in user_locks.locks.iter().enumerate() {
            if !lock.unlocked && is_matured(lock.start, lock.period, &durations, clock.unix_timestamp as u64) {
                indices_to_unlock.push(index);
            }
        }

        require!(!indices_to_unlock.is_empty(), EnginesError::LockNotMatured);

        for &index in indices_to_unlock.iter().rev() {
            unlock_by_index(&mut ctx, index)?;
            unlocked_count = unlocked_count.checked_add(1).ok_or(EnginesError::MathOverflow)?;
        }

        emit!(UnlockedAllMatured {
            user: ctx.accounts.user.key(),
            unlocked_count,
        });

        Ok(())
    }

    /// Burn tokens to give (Burn-to-Give functionality)
    pub fn burn_to_give(ctx: Context<BurnToGiveContext>, amount: u64) -> Result<()> {
        require!(amount > 0, EnginesError::InvalidAmount);
        require!(!ctx.accounts.state.paused, EnginesError::Paused);

        let clock = Clock::get()?;
        let user_burn_data = &mut ctx.accounts.user_burn_data;

        let cooldown_period = 7200;
        if user_burn_data.last_burn_timestamp > 0 {
            require!(
                clock.unix_timestamp as u64 >= user_burn_data.last_burn_timestamp + cooldown_period,
                EnginesError::WaitForCooldown
            );
        }

        let state_info = ctx.accounts.state.to_account_info();
        let state = &mut ctx.accounts.state;

        let available = available_excess(
            ctx.accounts.vault.amount,
            state.total_locked,
            state.reserve_for_donation,
        );
        require!(
            available >= amount,
            EnginesError::InsufficientMatchingTreasury
        );

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.burn_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        let bump = state.bump;
        let seeds: &[&[u8]] = &[
            b"engines_state".as_ref(),
            state.authority.as_ref(),
            &[bump],
        ];
        let signer_seeds: &[&[&[u8]]] = &[seeds];

        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.zcw_token_account.to_account_info(),
            authority: state_info.clone(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::transfer(cpi_ctx, amount)?;

        user_burn_data.owner = ctx.accounts.user.key();
        user_burn_data.total_burned = user_burn_data.total_burned
            .checked_add(amount)
            .ok_or(EnginesError::MathOverflow)?;
        user_burn_data.last_burn_timestamp = clock.unix_timestamp as u64;

        let mut badge_id = 0u64;
        if amount >= 10000 {
            badge_id = state.next_badge_id;
            state.next_badge_id = state.next_badge_id
                .checked_add(1)
                .ok_or(EnginesError::MathOverflow)?;

            user_burn_data.burn_badge_count = user_burn_data
                .burn_badge_count
                .checked_add(1)
                .ok_or(EnginesError::MathOverflow)?;
        }

        emit!(BurnToGiveEvent {
            user: ctx.accounts.user.key(),
            amount,
            timestamp: clock.unix_timestamp,
            badge_id,
        });

        emit!(MirroredDonation {
            founder: ctx.accounts.state.key(),
            amount,
            zakat_pool: ctx.accounts.state.zcw,
        });

        Ok(())
    }

    /// Set ZCW address (owner only)
    pub fn set_zcw(ctx: Context<SetZcw>, new_zcw: Pubkey) -> Result<()> {
        let state = &mut ctx.accounts.state;
        let old_zcw = state.zcw;
        state.zcw = new_zcw;

        emit!(ZcwUpdated {
            old_wallet: old_zcw,
            new_wallet: new_zcw,
        });

        Ok(())
    }

    /// Set donation rate for a specific period (owner only)
    pub fn set_donation_rate(ctx: Context<OnlyAuthority>, period: Period, new_rate: u16) -> Result<()> {
        let state = &mut ctx.accounts.state;
        let old_rate = state.donation_rates[period as usize];
        state.donation_rates[period as usize] = new_rate;

        emit!(DonationRateUpdated {
            period,
            old_rate,
            new_rate,
        });

        Ok(())
    }

    /// Pause/unpause the contract (owner only)
    pub fn set_paused(ctx: Context<OnlyAuthority>, paused: bool) -> Result<()> {
        ctx.accounts.state.paused = paused;
        Ok(())
    }

    /// Withdraw excess tokens (owner only)
    pub fn withdraw_excess(ctx: Context<WithdrawExcess>, amount: u64, _to: Pubkey) -> Result<()> {
        let state_info = ctx.accounts.state.to_account_info();
        let state = &ctx.accounts.state;

        let available = available_excess(
            ctx.accounts.vault.amount,
            state.total_locked,
            state.reserve_for_donation,
        );

        require!(
            amount <= available,
            EnginesError::InsufficientMatchingTreasury
        );

        let bump = state.bump;
        let seeds: &[&[u8]] = &[
            b"engines_state".as_ref(),
            state.authority.as_ref(),
            &[bump],
        ];
        let signer_seeds: &[&[&[u8]]] = &[seeds];

        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.to_token_account.to_account_info(),
            authority: state_info.clone(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }
}

/// Helper: unlock by index (with NFT metadata selection)
fn unlock_by_index(ctx: &mut Context<UnlockTokens>, index: usize) -> Result<()> {
    let clock = Clock::get()?;
    let state_info = ctx.accounts.state.to_account_info();
    let state = &mut ctx.accounts.state;

    {
        let user_locks = &mut ctx.accounts.user_data;
        require!(index < user_locks.locks.len(), EnginesError::BadLockId);

        {
            let lock = &mut user_locks.locks[index];
            require!(!lock.unlocked, EnginesError::AlreadyUnlocked);
            require!(
                is_matured(
                    lock.start,
                    lock.period,
                    &state.period_durations,
                    clock.unix_timestamp as u64
                ),
                EnginesError::LockNotMatured
            );

            let rate = state.donation_rates[lock.period as usize];
            let donation = (lock.amount as u128 * rate as u128) / state.scale as u128;
            let donation_u64 =
                u64::try_from(donation).map_err(|_| EnginesError::AmountTooLarge)?;
            let to_user = lock
                .amount
                .checked_sub(donation_u64)
                .ok_or(EnginesError::MathOverflow)?;

            lock.unlocked = true;
            state.total_locked = state
                .total_locked
                .checked_sub(lock.amount)
                .ok_or(EnginesError::MathOverflow)?;
            state.total_unlocked = state
                .total_unlocked
                .checked_add(to_user)
                .ok_or(EnginesError::MathOverflow)?;

            let authority_key = state.authority;
            let bump = state.bump;
            let seeds: &[&[u8]] =
                &[b"engines_state".as_ref(), authority_key.as_ref(), &[bump]];
            let signer_seeds: &[&[&[u8]]] = &[seeds];

            let mut matched = false;
            let mut match_amount = 0u64;

            // Matching donation
            if matches!(lock.period, Period::D180 | Period::D355) {
                let available = free_treasury(ctx.accounts.vault.amount, state.total_locked);
                require!(
                    available >= donation_u64,
                    EnginesError::InsufficientMatchingTreasury
                );

                let cpi_accounts = Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.zcw_token_account.to_account_info(),
                    authority: state_info.clone(),
                };
                let cpi_program = ctx.accounts.token_program.to_account_info();
                let cpi_ctx =
                    CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
                token::transfer(cpi_ctx, donation_u64)?;

                state.reserve_for_donation = state
                    .reserve_for_donation
                    .checked_sub(donation_u64)
                    .ok_or(EnginesError::MathOverflow)?;

                matched = true;
                match_amount = donation_u64;
            }

            // Send donation (user share)
            {
                let cpi_accounts = Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.zcw_token_account.to_account_info(),
                    authority: state_info.clone(),
                };
                let cpi_program = ctx.accounts.token_program.to_account_info();
                let cpi_ctx =
                    CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
                token::transfer(cpi_ctx, donation_u64)?;
            }

            // Return remaining tokens to user
            {
                let cpi_accounts = Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: state_info.clone(),
                };
                let cpi_program = ctx.accounts.token_program.to_account_info();
                let cpi_ctx =
                    CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
                token::transfer(cpi_ctx, to_user)?;
            }

            // Update per-user counters (drop lock borrow first)
            let donation_for_user = donation_u64;
            let lock_id = lock.id;
            drop(lock);

            let user_locks_outer = &mut ctx.accounts.user_data;
            user_locks_outer.total_donated = user_locks_outer
                .total_donated
                .checked_add(donation_for_user)
                .ok_or(EnginesError::MathOverflow)?;
            user_locks_outer.htg_badge_count = user_locks_outer
                .htg_badge_count
                .checked_add(1)
                .ok_or(EnginesError::MathOverflow)?;

            // === Mint NFT Badge with milestone-based URI ===
            let badge_id = state.next_badge_id;
            state.next_badge_id = state
                .next_badge_id
                .checked_add(1)
                .ok_or(EnginesError::MathOverflow)?;

            // choose URI based on lock.period
            let uri: String = match ctx.accounts.user_data.locks[index].period {
                Period::D30 => state.uri_30_days.clone(),
                Period::D60 => state.uri_60_days.clone(),
                Period::D180 => state.uri_180_days.clone(),
                Period::D355 => state.uri_365_days.clone(),
            };

            // Create metadata account (Metaplex)
            let cpi_accounts = CreateMetadataAccountsV3 {
                metadata: ctx.accounts.metadata_account.to_account_info(),
                mint: ctx.accounts.badge_mint.to_account_info(),
                mint_authority: state_info.clone(),
                update_authority: state_info.clone(),
                payer: ctx.accounts.user.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            };
            let cpi_program = ctx.accounts.metadata_program.to_account_info();
            let cpi_ctx =
                CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

            create_metadata_accounts_v3(
                cpi_ctx,
                DataV2 {
                    name: format!("HTG Badge #{}", badge_id),
                    symbol: "HTGB".to_string(),
                    uri,
                    seller_fee_basis_points: 0,
                    creators: None,
                    collection: None,
                    uses: None,
                },
                true,
                true,
                None,
            )?;

            // Mint 1 NFT to user's ATA
            let mint_accounts = MintTo {
                mint: ctx.accounts.badge_mint.to_account_info(),
                to: ctx.accounts.user_badge_token_account.to_account_info(),
                authority: state_info.clone(),
            };
            let mint_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                mint_accounts,
                signer_seeds,
            );
            token::mint_to(mint_ctx, 1)?;

            emit!(TokensUnlocked {
                user: ctx.accounts.user.key(),
                lock_id,
                returned_to_user: to_user,
                donation_to_zcw: donation_for_user,
                matched,
                match_amount,
                badge_id,
            });
        }
    }
    Ok(())
}

/// Helper functions
fn is_matured(start: u64, period: Period, durations: &[u64; 4], current_time: u64) -> bool {
    current_time >= start + durations[period as usize]
}

fn free_treasury(vault_balance: u64, total_locked: u64) -> u64 {
    if vault_balance > total_locked {
        vault_balance - total_locked
    } else {
        0
    }
}

fn available_excess(vault_balance: u64, total_locked: u64, reserve_for_donation: u64) -> u64 {
    let after_locked = if vault_balance > total_locked {
        vault_balance - total_locked
    } else {
        0
    };

    if after_locked > reserve_for_donation {
        after_locked - reserve_for_donation
    } else {
        0
    }
}

// Account Structures
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub zktc_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        space = EnginesState::SPACE,
        seeds = [b"engines_state", authority.key().as_ref()],
        bump
    )]
    pub state: Account<'info, EnginesState>,

    #[account(
        init,
        payer = authority,
        token::mint = zktc_mint,
        token::authority = state,
        seeds = [b"engines_vault", state.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct LockTokens<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub state: Account<'info, EnginesState>,

    #[account(
        init_if_needed,
        payer = user,
        space = UserLockData::SPACE,
        seeds = [b"user_locks", user.key().as_ref()],
        bump
    )]
    pub user_data: Account<'info, UserLockData>,

    #[account(
        mut,
        constraint = user_token_account.mint == state.zktc_mint,
        constraint = user_token_account.owner == user.key()
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = vault.mint == state.zktc_mint
    )]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UnlockTokens<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, has_one = authority)]
    pub state: Account<'info, EnginesState>,

    #[account(
        mut,
        seeds = [b"user_locks", user.key().as_ref()],
        bump
    )]
    pub user_data: Account<'info, UserLockData>,

    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub zcw_token_account: Account<'info, TokenAccount>,

    /// Badge NFT mint (must be provided / derived by client)
    #[account(mut)]
    pub badge_mint: Account<'info, Mint>,

    /// User’s ATA for the badge NFT
    #[account(mut)]
    pub user_badge_token_account: Account<'info, TokenAccount>,

    /// Metadata PDA for badge NFT (UncheckedAccount)
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,

    /// Programs
    pub token_program: Program<'info, Token>,
    pub metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    /// Authority signer PDA
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct BurnToGiveContext<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub state: Account<'info, EnginesState>,

    #[account(
        init_if_needed,
        payer = user,
        space = UserBurnData::SPACE,
        seeds = [b"user_burn", user.key().as_ref()],
        bump
    )]
    pub user_burn_data: Account<'info, UserBurnData>,

    #[account(
        mut,
        constraint = user_token_account.mint == state.zktc_mint,
        constraint = user_token_account.owner == user.key()
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = vault.mint == state.zktc_mint
    )]
    pub vault: Account<'info, TokenAccount>,

    /// Burn vault (dead address equivalent) - Fixed to use the mint account
    #[account(
        init_if_needed,
        payer = user,
        token::mint = zktc_mint,
        token::authority = user,
        seeds = [b"burn_vault"],
        bump
    )]
    pub burn_vault: Account<'info, TokenAccount>,

    /// The mint account reference
    pub zktc_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = zcw_token_account.mint == state.zktc_mint
    )]
    pub zcw_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct OnlyAuthority<'info> {
    #[account(mut, has_one = authority)]
    pub state: Account<'info, EnginesState>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetZcw<'info> {
    #[account(mut, has_one = authority)]
    pub state: Account<'info, EnginesState>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct WithdrawExcess<'info> {
    #[account(mut, has_one = authority)]
    pub state: Account<'info, EnginesState>,
    pub authority: Signer<'info>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub to_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

// State Structures
#[account]
pub struct EnginesState {
    pub authority: Pubkey,
    pub zktc_mint: Pubkey,
    pub zcw: Pubkey,
    pub vault: Pubkey,
    pub total_locked: u64,
    pub total_unlocked: u64,
    pub next_badge_id: u64,
    pub next_lock_id: u64,
    pub reserve_for_donation: u64,
    pub paused: bool,
    pub donation_rates: [u16; 4],
    pub scale: u16,
    pub period_durations: [u64; 4],
    pub bump: u8,
    pub uri_30_days: String,
    pub uri_60_days: String,
    pub uri_180_days: String,
    pub uri_365_days: String,
}

impl EnginesState {
    // Space calculation includes a safe buffer for the 4 URI strings (max 200 bytes each)
    pub const SPACE: usize = 8 + // discriminator
        32 + // authority
        32 + // zktc_mint
        32 + // zcw
        32 + // vault
        8 + // total_locked
        8 + // total_unlocked
        8 + // next_badge_id
        8 + // next_lock_id
        8 + // reserve_for_donation
        1 + // paused
        8 + // donation_rates (4 * u16)
        2 + // scale
        32 + // period_durations (4 * u64)
        1 + // bump
        (4 * (4 + 200)) + // 4 URIs: each has 4 bytes length + up to 200 bytes content
        64; // extra padding
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct LockPosition {
    pub id: u64,
    pub amount: u64,
    pub start: u64,
    pub period: Period,
    pub unlocked: bool,
}

pub const MAX_LOCKS_PER_USER: usize = 50;

#[account]
pub struct UserLockData {
    pub owner: Pubkey,
    pub locks: Vec<LockPosition>,
    pub total_donated: u64,
    pub htg_badge_count: u64,
}

impl UserLockData {
    // conservative sizing
    pub const EST_LOCK_SIZE: usize = 72; // estimated per-lock bytes (id+amount+start+period+bool + padding)
    pub const SPACE: usize = 8 + // discriminator
        32 + // owner
        4 + (MAX_LOCKS_PER_USER * Self::EST_LOCK_SIZE) + // locks vector
        8 + // total_donated
        8; // htg_badge_count
}

#[account]
pub struct UserBurnData {
    pub owner: Pubkey,
    pub last_burn_timestamp: u64,
    pub burn_badge_count: u64,
    pub total_burned: u64,
}

impl UserBurnData {
    pub const SPACE: usize = 8 + // discriminator
        32 + // owner
        8 + // last_burn_timestamp
        8 + // burn_badge_count
        8; // total_burned
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum Period {
    D30,
    D60,
    D180,
    D355,
}

// Error Types
#[error_code]
pub enum EnginesError {
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Already unlocked")]
    AlreadyUnlocked,
    #[msg("Lock not matured")]
    LockNotMatured,
    #[msg("Bad lock ID")]
    BadLockId,
    #[msg("Insufficient matching treasury")]
    InsufficientMatchingTreasury,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Amount too large")]
    AmountTooLarge,
    #[msg("Contract is paused")]
    Paused,
    #[msg("Wait for cooldown")]
    WaitForCooldown,
    #[msg("Too many locks per user")]
    TooManyLocks,
}

// Events
#[event]
pub struct Initialized {
    pub authority: Pubkey,
    pub zktc_mint: Pubkey,
    pub zcw: Pubkey,
}

#[event]
pub struct TokensLocked {
    pub user: Pubkey,
    pub lock_id: u64,
    pub amount: u64,
    pub period: Period,
    pub timestamp: i64,
}

#[event]
pub struct TokensUnlocked {
    pub user: Pubkey,
    pub lock_id: u64,
    pub returned_to_user: u64,
    pub donation_to_zcw: u64,
    pub matched: bool,
    pub match_amount: u64,
    pub badge_id: u64,
}

#[event]
pub struct UnlockedAllMatured {
    pub user: Pubkey,
    pub unlocked_count: u32,
}

#[event]
pub struct BurnToGiveEvent {
    pub user: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
    pub badge_id: u64,
}

#[event]
pub struct MirroredDonation {
    pub founder: Pubkey,
    pub amount: u64,
    pub zakat_pool: Pubkey,
}

#[event]
pub struct ZcwUpdated {
    pub old_wallet: Pubkey,
    pub new_wallet: Pubkey,
}

#[event]
pub struct DonationRateUpdated {
    pub period: Period,
    pub old_rate: u16,
    pub new_rate: u16,
}
