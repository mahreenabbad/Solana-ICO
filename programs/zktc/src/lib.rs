use anchor_lang::prelude::*;
use anchor_spl::token::{
    self, Token, Mint, TokenAccount, MintTo, Burn,
};

declare_id!("9kV35dMKi9azmFAiibSUkyMFoZ3QikfE9BV12dN6sgaK");

#[program]
pub mod zktc {
    use super::*;

    // Initialize the state with an authority (owner)
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.owner = ctx.accounts.authority.key();
        Ok(())
    }

    // Mint tokens (only owner can call)
    pub fn mint(ctx: Context<MintToken>, amount: u64) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.state.owner,
            ctx.accounts.authority.key(),
            ZakatError::Unauthorized
        );

        // Additional safety check: ensure amount is not zero
        require!(amount > 0, ZakatError::InvalidAmount);

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.token_account.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        );
        token::mint_to(cpi_ctx, amount)
    }

    // Burn tokens (only owner can call)
    pub fn burn(ctx: Context<BurnToken>, amount: u64) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.state.owner,
            ctx.accounts.authority.key(),
            ZakatError::Unauthorized
        );

        // Additional safety check: ensure amount is not zero
        require!(amount > 0, ZakatError::InvalidAmount);

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.mint.to_account_info(),
                from: ctx.accounts.token_account.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        );
        token::burn(cpi_ctx, amount)
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + 32)] // works for keypair
    pub state: Account<'info, State>,
    /// CHECK: This is the mint account - validated by SPL token program
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct MintToken<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,

    /// CHECK: SPL Token Mint account - validated by SPL token program
    #[account(mut)]
    pub mint: Account<'info, Mint>,

    /// CHECK: SPL Token Account - validated by SPL token program  
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct BurnToken<'info> {
    #[account(mut)]
    pub state: Account<'info, State>,

    /// CHECK: SPL Token Mint account - validated by SPL token program
    #[account(mut)]
    pub mint: Account<'info, Mint>,

    /// CHECK: SPL Token Account - validated by SPL token program
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub authority: Signer<'info>,
}

#[account]
pub struct State {
    pub owner: Pubkey,
}

#[error_code]
pub enum ZakatError {
    #[msg("You are not authorized to perform this action")]
    Unauthorized,
    #[msg("Invalid amount: must be greater than zero")]
    InvalidAmount,
    #[msg("Insufficient token balance")]
    InsufficientBalance,
}

// Program Id: 9kV35dMKi9azmFAiibSUkyMFoZ3QikfE9BV12dN6sgaK

// Signature: asAbztRZKHUPbfLFyfdpjXq1oShs1knocmqVqZ8N8rLJHnJ2UVBfQXLrKRdTQwSLmhaPFGywN1xJRgJNQkVZXz5
