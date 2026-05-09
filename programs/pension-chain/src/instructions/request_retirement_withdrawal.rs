use anchor_lang::prelude::*;

use crate::errors::PensionError;
use crate::state::{
    AffiliateAccount, FundConfig, InvestmentPool, WithdrawalMode, WithdrawalReason,
    WithdrawalRequest, WithdrawalStatus,
};

#[derive(Accounts)]
#[instruction(request_nonce: u32)]
pub struct RequestRetirementWithdrawal<'info> {
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
        seeds = [b"pool", &[affiliate_account.risk_profile as u8]],
        bump = pool.bump,
    )]
    pub pool: Account<'info, InvestmentPool>,

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
    ctx: Context<RequestRetirementWithdrawal>,
    request_nonce: u32,
    amount_requested: u64,
    withdrawal_mode: WithdrawalMode,
) -> Result<()> {
    let cfg = &ctx.accounts.fund_config;
    let acc = &mut ctx.accounts.affiliate_account;
    let clock = Clock::get()?;

    // Validate withdrawal mode is set.
    require!(
        withdrawal_mode != WithdrawalMode::NotSet,
        PensionError::WithdrawalModeNotSet
    );

    // Validate retirement age.
    let current_age = acc.age_at(clock.unix_timestamp);
    require!(
        current_age >= cfg.min_retirement_age,
        PensionError::NotRetirementAge
    );

    // Validate minimum contribution years.
    require!(
        acc.contribution_years() >= cfg.min_contribution_years as u32,
        PensionError::InsufficientContributionYears
    );

    let total_balance = acc.total_contributed
        .checked_add(acc.accrued_yield)
        .ok_or(PensionError::ContributionOverflow)?;

    require!(amount_requested <= total_balance, PensionError::InsufficientFunds);

    // Enforce annual withdrawal cap (15% of balance per year).
    let current_year = (clock.unix_timestamp / (365 * 24 * 3600)) as i16;
    if acc.withdrawal_year == current_year {
        let max_this_year = total_balance
            .checked_mul(cfg.max_annual_withdrawal_bps as u64)
            .ok_or(PensionError::ContributionOverflow)?
            / 10_000;
        require!(
            acc.withdrawn_this_year.saturating_add(amount_requested) <= max_this_year,
            PensionError::AnnualCapExceeded
        );
    }

    // Mark as retired and set withdrawal mode.
    acc.retired = true;
    acc.withdrawal_mode = withdrawal_mode;

    // Create the withdrawal request (status = Pending, approved by keeper).
    let req = &mut ctx.accounts.withdrawal_request;
    req.affiliate               = ctx.accounts.affiliate.key();
    req.request_nonce           = request_nonce;
    req.reason                  = WithdrawalReason::Retirement;
    req.status                  = WithdrawalStatus::Pending;
    req.amount_requested        = amount_requested;
    req.amount_released         = 0;
    req.penalty_amount          = 0;
    req.requested_at            = clock.unix_timestamp;
    req.resolved_at             = 0;
    req.oracle_attestation_hash = [0u8; 32];
    req.beneficiary_wallet      = None;
    req.bump                    = ctx.bumps.withdrawal_request;

    msg!(
        "Retirement withdrawal requested: {} tokens | mode: {:?} | age: {}",
        amount_requested,
        withdrawal_mode as u8,
        current_age
    );

    Ok(())
}