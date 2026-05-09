use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::errors::PensionError;
use crate::state::{FundConfig, InvestmentPool, RiskProfile};

#[derive(Accounts)]
#[instruction(risk_profile: RiskProfile)]
pub struct InitializePool<'info> {
    #[account(
        seeds = [b"fund_config"],
        bump = fund_config.bump,
        constraint = fund_config.authority == authority.key() @ PensionError::Unauthorized,
    )]
    pub fund_config: Account<'info, FundConfig>,

    #[account(
        init,
        payer = authority,
        space = InvestmentPool::LEN,
        seeds = [&b"pool"[..], &[risk_profile as u8][..]],
        bump
    )]
    pub pool: Account<'info, InvestmentPool>,

    /// Token account that will hold this pool's liquid balance.
    #[account(
        constraint = pool_token_account.mint == fund_config.accepted_mint @ PensionError::PoolMismatch,
    )]
    pub pool_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<InitializePool>, risk_profile: RiskProfile) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    pool.risk_profile     = risk_profile;
    pool.token_account    = ctx.accounts.pool_token_account.key();
    pool.total_principal  = 0;
    pool.total_yield      = 0;
    pool.current_apy_bps  = 0;
    pool.last_harvest_at  = Clock::get()?.unix_timestamp;
    pool.affiliate_count  = 0;
    pool.allocation_bps   = InvestmentPool::default_allocation(risk_profile);
    pool.bump             = ctx.bumps.pool;

    msg!("Pool initialized for risk profile: {:?}", risk_profile as u8);
    Ok(())
}