use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::PensionError;
use crate::state::{
    AffiliateAccount, FundConfig, InvestmentPool, WithdrawalReason, WithdrawalRequest,
    WithdrawalStatus,
};

#[derive(Accounts)]
#[instruction(request_nonce: u32, beneficiary_index: u8)]
pub struct ProcessInheritance<'info> {
    #[account(
        seeds = [b"fund_config"],
        bump = fund_config.bump,
        constraint = !fund_config.paused @ PensionError::FundPaused,
        constraint = fund_config.authority == keeper.key() @ PensionError::Unauthorized,
    )]
    pub fund_config: Account<'info, FundConfig>,

    #[account(
        mut,
        seeds = [b"affiliate", affiliate_account.owner.as_ref()],
        bump = affiliate_account.bump,
        constraint = affiliate_account.deceased @ PensionError::NotDeceased,
    )]
    pub affiliate_account: Account<'info, AffiliateAccount>,

    #[account(
        mut,
        seeds = [b"pool", &[affiliate_account.risk_profile as u8]],
        bump = pool.bump,
    )]
    pub pool: Account<'info, InvestmentPool>,

    #[account(
        init,
        payer = keeper,
        space = WithdrawalRequest::LEN,
        seeds = [
            b"withdrawal",
            affiliate_account.owner.as_ref(),
            &request_nonce.to_le_bytes()
        ],
        bump
    )]
    pub withdrawal_request: Account<'info, WithdrawalRequest>,

    #[account(
        mut,
        constraint = pool_token_account.key() == pool.token_account @ PensionError::PoolMismatch,
    )]
    pub pool_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = beneficiary_token_account.mint == fund_config.accepted_mint @ PensionError::PoolMismatch,
    )]
    pub beneficiary_token_account: Account<'info, TokenAccount>,

    /// CHECK: Validated via seeds.
    #[account(
        seeds = [b"fund_config"],
        bump = fund_config.bump,
    )]
    pub fund_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub keeper: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(
    ctx: Context<ProcessInheritance>,
    request_nonce: u32,
    beneficiary_index: u8,
    oracle_attestation_hash: [u8; 32],
) -> Result<()> {
    let cfg = &ctx.accounts.fund_config;
    let acc = &mut ctx.accounts.affiliate_account;
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;

    require!(!acc.beneficiaries.is_empty(), PensionError::NoBeneficiaries);
    require!(
        oracle_attestation_hash != [0u8; 32],
        PensionError::MissingAttestation
    );

    let beneficiary_index = beneficiary_index as usize;
    require!(
        beneficiary_index < acc.beneficiaries.len(),
        PensionError::BeneficiaryNotFound
    );

    let beneficiary = acc.beneficiaries[beneficiary_index].clone();

    // Validate the recipient token account belongs to the registered beneficiary.
    require!(
        ctx.accounts.beneficiary_token_account.owner == beneficiary.wallet,
        PensionError::BeneficiaryNotFound
    );

    // Compute this beneficiary's share of the total balance.
    let total_balance = acc.total_contributed
        .checked_add(acc.accrued_yield)
        .ok_or(PensionError::ContributionOverflow)?;

    let payout = total_balance
        .checked_mul(beneficiary.share_bps as u64)
        .ok_or(PensionError::ContributionOverflow)?
        / 10_000;

    // Deduct from balances.
    let from_principal = payout.min(acc.total_contributed);
    let from_yield = payout.saturating_sub(from_principal);
    acc.total_contributed = acc.total_contributed.saturating_sub(from_principal);
    acc.accrued_yield = acc.accrued_yield.saturating_sub(from_yield);
    pool.total_principal = pool.total_principal.saturating_sub(from_principal);
    pool.total_yield = pool.total_yield.saturating_sub(from_yield);

    // Transfer to beneficiary.
    let seeds = &[b"fund_config".as_ref(), &[cfg.bump]];
    let signer_seeds = &[&seeds[..]];

    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from:      ctx.accounts.pool_token_account.to_account_info(),
            to:        ctx.accounts.beneficiary_token_account.to_account_info(),
            authority: ctx.accounts.fund_authority.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(transfer_ctx, payout)?;

    // Record the inheritance withdrawal.
    let req = &mut ctx.accounts.withdrawal_request;
    req.affiliate               = acc.owner;
    req.request_nonce           = request_nonce;
    req.reason                  = WithdrawalReason::Inheritance;
    req.status                  = WithdrawalStatus::Executed;
    req.amount_requested        = payout;
    req.amount_released         = payout;
    req.penalty_amount          = 0;
    req.requested_at            = clock.unix_timestamp;
    req.resolved_at             = clock.unix_timestamp;
    req.oracle_attestation_hash = oracle_attestation_hash;
    req.beneficiary_wallet      = Some(beneficiary.wallet);
    req.bump                    = ctx.bumps.withdrawal_request;

    msg!(
        "Inheritance payout: {} tokens to beneficiary {} ({} bps share)",
        payout,
        beneficiary.wallet,
        beneficiary.share_bps
    );

    Ok(())
}