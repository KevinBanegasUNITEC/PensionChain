use anchor_lang::prelude::*;

use crate::errors::PensionError;
use crate::state::{AffiliateAccount, FundConfig, InvestmentPool};

/// Minimum seconds between keeper harvests (24 hours).
pub const MIN_HARVEST_INTERVAL: i64 = 86_400;

#[derive(Accounts)]
pub struct HarvestYield<'info> {
    #[account(
        seeds = [b"fund_config"],
        bump = fund_config.bump,
        constraint = !fund_config.paused @ PensionError::FundPaused,
        constraint = fund_config.authority == keeper.key() @ PensionError::Unauthorized,
    )]
    pub fund_config: Account<'info, FundConfig>,

    #[account(
        mut,
        seeds = [b"pool", &[pool.risk_profile as u8]],
        bump = pool.bump,
    )]
    pub pool: Account<'info, InvestmentPool>,

    #[account(
        mut,
        seeds = [b"affiliate", affiliate_account.owner.as_ref()],
        bump = affiliate_account.bump,
        constraint = !affiliate_account.deceased @ PensionError::AccountDeceased,
        constraint = affiliate_account.risk_profile == pool.risk_profile @ PensionError::PoolMismatch,
    )]
    pub affiliate_account: Account<'info, AffiliateAccount>,

    #[account(mut)]
    pub keeper: Signer<'info>,
}

pub fn handler(
    ctx: Context<HarvestYield>,
    new_apy_bps: u16,
    yield_amount: u64,
) -> Result<()> {
    let cfg = &ctx.accounts.fund_config;
    let pool = &mut ctx.accounts.pool;
    let acc = &mut ctx.accounts.affiliate_account;
    let clock = Clock::get()?;

    require!(
        clock.unix_timestamp - pool.last_harvest_at >= MIN_HARVEST_INTERVAL,
        PensionError::HarvestTooEarly
    );

    // Apply protocol fee on yield before crediting.
    let fee = yield_amount
        .checked_mul(cfg.protocol_fee_bps as u64)
        .ok_or(PensionError::ContributionOverflow)?
        / 10_000;
    let net_yield = yield_amount.saturating_sub(fee);

    // Credit net yield to affiliate.
    acc.accrued_yield = acc.accrued_yield
        .checked_add(net_yield)
        .ok_or(PensionError::ContributionOverflow)?;

    // Update pool metrics.
    pool.total_yield = pool.total_yield
        .checked_add(net_yield)
        .ok_or(PensionError::ContributionOverflow)?;
    pool.current_apy_bps = new_apy_bps;
    pool.last_harvest_at = clock.unix_timestamp;

    msg!(
        "Yield harvested: {} tokens (fee: {}) | new APY: {}bps | affiliate total yield: {}",
        net_yield,
        fee,
        new_apy_bps,
        acc.accrued_yield
    );

    Ok(())
}