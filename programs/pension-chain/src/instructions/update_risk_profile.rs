use anchor_lang::prelude::*;

use crate::errors::PensionError;
use crate::state::{AffiliateAccount, FundConfig, InvestmentPool, RiskProfile};

/// Minimum seconds between risk profile changes (6 months ≈ 180 days).
const MIN_PROFILE_CHANGE_INTERVAL: i64 = 180 * 24 * 3600;

#[derive(Accounts)]
#[instruction(
    new_risk_profile: RiskProfile,
    advisor_session_hash: [u8; 32]
)]
pub struct UpdateRiskProfile<'info> {
    #[account(
        seeds = [b"fund_config"],
        bump = fund_config.bump,
        constraint = !fund_config.paused @ PensionError::FundPaused,
    )]
    pub fund_config: Account<'info, FundConfig>,

    #[account(
        mut,
        seeds = [b"affiliate", affiliate.key().as_ref()],
        bump = affiliate_account.bump,
        constraint = !affiliate_account.deceased @ PensionError::AccountDeceased,
        constraint = affiliate_account.owner == affiliate.key() @ PensionError::Unauthorized,
    )]
    pub affiliate_account: Account<'info, AffiliateAccount>,

    /// Current pool (being left).
    #[account(
        mut,
        seeds = [b"pool", &[affiliate_account.risk_profile as u8]],
        bump = old_pool.bump,
    )]
    pub old_pool: Account<'info, InvestmentPool>,

    /// New pool (being joined).
    #[account(
        mut,
        seeds = [&b"pool"[..], &[new_risk_profile as u8][..]],
        bump = new_pool.bump,
    )]
    pub new_pool: Account<'info, InvestmentPool>,

    #[account(mut)]
    pub affiliate: Signer<'info>,
}

pub fn handler(
    ctx: Context<UpdateRiskProfile>,
    new_risk_profile: RiskProfile,
    advisor_session_hash: [u8; 32],
) -> Result<()> {
    let acc = &mut ctx.accounts.affiliate_account;
    let clock = Clock::get()?;

    // Enforce 6-month cooldown.
    if acc.enrolled_at > 0 && acc.last_contribution_at > 0 {
        // Use last_contribution_at as a proxy for last profile change timestamp.
        // In production you'd store a dedicated `last_profile_change_at` field.
        require!(
            clock.unix_timestamp - acc.last_contribution_at >= MIN_PROFILE_CHANGE_INTERVAL,
            PensionError::PeriodNotElapsed
        );
    }

    let old_profile = acc.risk_profile;

    // Move affiliate count between pools.
    ctx.accounts.old_pool.affiliate_count =
        ctx.accounts.old_pool.affiliate_count.saturating_sub(1);
    ctx.accounts.new_pool.affiliate_count = ctx.accounts
        .new_pool
        .affiliate_count
        .checked_add(1)
        .ok_or(PensionError::ContributionOverflow)?;

    // Update principal tracking (conceptual — actual token rebalancing is
    // done by the keeper off-chain via separate rebalance instruction).
    ctx.accounts.old_pool.total_principal =
        ctx.accounts.old_pool.total_principal.saturating_sub(acc.total_contributed);
    ctx.accounts.new_pool.total_principal = ctx.accounts
        .new_pool
        .total_principal
        .checked_add(acc.total_contributed)
        .ok_or(PensionError::ContributionOverflow)?;

    acc.risk_profile = new_risk_profile;
    acc.advisor_session_hash = advisor_session_hash;

    msg!(
        "Risk profile updated: {:?} → {:?} | new advisor session: {:?}",
        old_profile as u8,
        new_risk_profile as u8,
        &advisor_session_hash[..4]
    );

    Ok(())
}