use anchor_lang::prelude::*;

use crate::errors::PensionError;
use crate::state::{
    AffiliateAccount, FundConfig, WithdrawalReason, WithdrawalRequest, WithdrawalStatus,
};

#[derive(Accounts)]
#[instruction(request_nonce: u32)]
pub struct RequestEmergencyWithdrawal<'info> {
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
    ctx: Context<RequestEmergencyWithdrawal>,
    request_nonce: u32,
    reason: WithdrawalReason,
    oracle_attestation_hash: [u8; 32],
) -> Result<()> {
    let cfg = &ctx.accounts.fund_config;
    let acc = &ctx.accounts.affiliate_account;
    let clock = Clock::get()?;

    // Only medical emergency and natural disaster are valid here.
    require!(
        reason == WithdrawalReason::MedicalEmergency
            || reason == WithdrawalReason::NaturalDisaster,
        PensionError::WithdrawalModeNotSet
    );

    // Oracle attestation hash must be non-zero — submitted by the affiliate
    // after off-chain oracle verification. The keeper validates the hash against
    // the oracle's signed response before approving.
    require!(
        oracle_attestation_hash != [0u8; 32],
        PensionError::MissingAttestation
    );

    let total_balance = acc.total_contributed
        .checked_add(acc.accrued_yield)
        .ok_or(PensionError::ContributionOverflow)?;

    // Calculate the 30% release amount from config.
    let release_bps = match reason {
        WithdrawalReason::MedicalEmergency => cfg.emergency_release_bps,
        WithdrawalReason::NaturalDisaster  => cfg.disaster_release_bps,
        _ => unreachable!(),
    };
    let amount_requested = total_balance
        .checked_mul(release_bps as u64)
        .ok_or(PensionError::ContributionOverflow)?
        / 10_000;

    let req = &mut ctx.accounts.withdrawal_request;
    req.affiliate               = ctx.accounts.affiliate.key();
    req.request_nonce           = request_nonce;
    req.reason                  = reason;
    req.status                  = WithdrawalStatus::Pending;
    req.amount_requested        = amount_requested;
    req.amount_released         = 0;
    req.penalty_amount          = 0;
    req.requested_at            = clock.unix_timestamp;
    req.resolved_at             = 0;
    req.oracle_attestation_hash = oracle_attestation_hash;
    req.beneficiary_wallet      = None;
    req.bump                    = ctx.bumps.withdrawal_request;

    msg!(
        "Emergency withdrawal requested: {} tokens | reason: {:?}",
        amount_requested,
        reason as u8
    );

    Ok(())
}