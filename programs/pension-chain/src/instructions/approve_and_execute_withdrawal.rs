use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::PensionError;
use crate::state::{
    AffiliateAccount, FundConfig, InvestmentPool, WithdrawalReason, WithdrawalRequest,
    WithdrawalStatus,
};

#[derive(Accounts)]
pub struct ApproveAndExecuteWithdrawal<'info> {
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
    )]
    pub affiliate_account: Account<'info, AffiliateAccount>,

    #[account(
        mut,
        seeds = [b"pool", &[affiliate_account.risk_profile as u8]],
        bump = pool.bump,
    )]
    pub pool: Account<'info, InvestmentPool>,

    #[account(
        mut,
        seeds = [
            b"withdrawal",
            affiliate_account.owner.as_ref(),
            &withdrawal_request.request_nonce.to_le_bytes()
        ],
        bump = withdrawal_request.bump,
        constraint = withdrawal_request.status == WithdrawalStatus::Pending
            @ PensionError::RequestNotPending,
        constraint = withdrawal_request.affiliate == affiliate_account.owner
            @ PensionError::Unauthorized,
    )]
    pub withdrawal_request: Account<'info, WithdrawalRequest>,

    /// Pool's source token account.
    #[account(
        mut,
        constraint = pool_token_account.key() == pool.token_account @ PensionError::PoolMismatch,
    )]
    pub pool_token_account: Account<'info, TokenAccount>,

    /// Recipient token account (affiliate or beneficiary).
    #[account(
        mut,
        constraint = recipient_token_account.mint == fund_config.accepted_mint @ PensionError::PoolMismatch,
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    /// The fund_config PDA is the authority over pool token accounts.
    #[account(
        seeds = [b"fund_config"],
        bump = fund_config.bump,
    )]
    /// CHECK: This is the PDA signer — validated via seeds.
    pub fund_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub keeper: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<ApproveAndExecuteWithdrawal>) -> Result<()> {
    let cfg = &ctx.accounts.fund_config;
    let acc = &mut ctx.accounts.affiliate_account;
    let pool = &mut ctx.accounts.pool;
    let req = &mut ctx.accounts.withdrawal_request;
    let clock = Clock::get()?;

    let amount = req.amount_requested;

    require!(amount > 0, PensionError::InsufficientFunds);

    // Deduct from affiliate balance — principal first, then yield.
    let from_principal = amount.min(acc.total_contributed);
    let from_yield = amount.saturating_sub(from_principal);

    acc.total_contributed = acc.total_contributed.saturating_sub(from_principal);
    acc.accrued_yield = acc.accrued_yield.saturating_sub(from_yield);

    // Update pool principal.
    pool.total_principal = pool.total_principal.saturating_sub(from_principal);
    pool.total_yield = pool.total_yield.saturating_sub(from_yield);

    // Track annual withdrawal for retirement cap enforcement.
    if req.reason == WithdrawalReason::Retirement {
        let current_year = (clock.unix_timestamp / (365 * 24 * 3600)) as i16;
        if acc.withdrawal_year != current_year {
            acc.withdrawn_this_year = 0;
            acc.withdrawal_year = current_year;
        }
        acc.withdrawn_this_year = acc.withdrawn_this_year.saturating_add(amount);
    }

    // Transfer tokens from pool to recipient using fund PDA as signer.
    let seeds = &[b"fund_config".as_ref(), &[cfg.bump]];
    let signer_seeds = &[&seeds[..]];

    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from:      ctx.accounts.pool_token_account.to_account_info(),
            to:        ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.fund_authority.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(transfer_ctx, amount)?;

    // Finalize request.
    req.status        = WithdrawalStatus::Executed;
    req.amount_released = amount;
    req.resolved_at   = clock.unix_timestamp;

    msg!(
        "Withdrawal executed: {} tokens | reason: {:?} | affiliate balance remaining: {}",
        amount,
        req.reason as u8,
        acc.total_contributed.saturating_add(acc.accrued_yield)
    );

    Ok(())
}