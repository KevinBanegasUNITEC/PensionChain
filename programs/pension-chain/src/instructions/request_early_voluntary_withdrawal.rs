use anchor_lang::prelude::*;

use crate::errors::PensionError;
use crate::state::{
    AffiliateAccount, FundConfig, WithdrawalReason, WithdrawalRequest, WithdrawalStatus,
};

/// Maximum fraction of balance that can be withdrawn early (10% in bps).
const EARLY_WITHDRAWAL_CAP_BPS: u64 = 1_000;

#[derive(Accounts)]
#[instruction(request_nonce: u32)]
pub struct RequestEarlyVoluntaryWithdrawal<'info> {
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

    #[account(
        init,
        payer = affiliate,
        space = WithdrawalRequest::LEN,
        seeds = [
            b"withdrawal",
            affiliate.key().as_ref(),
            &request_nonce.to_le_bytes()
        ],
        bump
    )]
    pub withdrawal_request: Account<'info, WithdrawalRequest>,

    #[account(mut)]
    pub affiliate: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<RequestEarlyVoluntaryWithdrawal>,
    request_nonce: u32,
) -> Result<()> {
    let cfg = &ctx.accounts.fund_config;
    let acc = &mut ctx.accounts.affiliate_account;
    let clock = Clock::get()?;

    // Block if already eligible for retirement.
    let current_age = acc.age_at(clock.unix_timestamp);
    require!(
        current_age < cfg.min_retirement_age
            || acc.contribution_years() < cfg.min_contribution_years as u32,
        PensionError::AlreadyEligibleForRetirement
    );

    let total_balance = acc.total_contributed
        .checked_add(acc.accrued_yield)
        .ok_or(PensionError::ContributionOverflow)?;

    // Only 10% of balance can be withdrawn early.
    let amount_requested = total_balance
        .checked_mul(EARLY_WITHDRAWAL_CAP_BPS)
        .ok_or(PensionError::ContributionOverflow)?
        / 10_000;

    // Penalty: reduce accrued yield by early_withdrawal_penalty_bps.
    let penalty = acc.accrued_yield
        .checked_mul(cfg.early_withdrawal_penalty_bps as u64)
        .ok_or(PensionError::ContributionOverflow)?
        / 10_000;

    // Apply penalty immediately to the affiliate's yield balance.
    acc.accrued_yield = acc.accrued_yield.saturating_sub(penalty);

    let req = &mut ctx.accounts.withdrawal_request;
    req.affiliate               = ctx.accounts.affiliate.key();
    req.request_nonce           = request_nonce;
    req.reason                  = WithdrawalReason::EarlyVoluntary;
    req.status                  = WithdrawalStatus::Pending;
    req.amount_requested        = amount_requested;
    req.amount_released         = 0;
    req.penalty_amount          = penalty;
    req.requested_at            = clock.unix_timestamp;
    req.resolved_at             = 0;
    req.oracle_attestation_hash = [0u8; 32];
    req.beneficiary_wallet      = None;
    req.bump                    = ctx.bumps.withdrawal_request;

    msg!(
        "Early voluntary withdrawal requested: {} tokens | penalty on yield: {}",
        amount_requested,
        penalty
    );

    Ok(())
}