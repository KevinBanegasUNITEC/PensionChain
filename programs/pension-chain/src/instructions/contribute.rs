use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::PensionError;
use crate::state::{AffiliateAccount, ContributionRecord, FundConfig, InvestmentPool};

#[derive(Accounts)]
#[instruction(period_index: u32)]
pub struct Contribute<'info> {
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

    /// The investment pool for the affiliate's risk profile.
    #[account(
        mut,
        seeds = [b"pool", &[affiliate_account.risk_profile as u8]],
        bump = pool.bump,
    )]
    pub pool: Account<'info, InvestmentPool>,

    /// Contribution record for this specific period. Initialized on first deposit.
    #[account(
        init_if_needed,
        payer = affiliate,
        space = ContributionRecord::LEN,
        seeds = [
            b"contribution",
            affiliate.key().as_ref(),
            &period_index.to_le_bytes()
        ],
        bump
    )]
    pub contribution_record: Account<'info, ContributionRecord>,

    /// Affiliate's source token account.
    #[account(
        mut,
        constraint = affiliate_token_account.owner == affiliate.key() @ PensionError::Unauthorized,
        constraint = affiliate_token_account.mint == fund_config.accepted_mint @ PensionError::PoolMismatch,
    )]
    pub affiliate_token_account: Account<'info, TokenAccount>,

    /// Pool's destination token account.
    #[account(
        mut,
        constraint = pool_token_account.key() == pool.token_account @ PensionError::PoolMismatch,
    )]
    pub pool_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub affiliate: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Contribute>, period_index: u32, amount: u64) -> Result<()> {
    let cfg = &ctx.accounts.fund_config;
    let acc = &mut ctx.accounts.affiliate_account;
    let record = &mut ctx.accounts.contribution_record;
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;

    require!(amount >= cfg.min_contribution, PensionError::BelowMinimumContribution);

    // Enforce period timing: the affiliate can only contribute once the
    // previous period has elapsed. Allow top-ups within the same period.
    if acc.last_contribution_at > 0 {
        let expected_period_start =
            acc.enrolled_at + (period_index as i64 * cfg.contribution_period);
        require!(
            clock.unix_timestamp >= expected_period_start,
            PensionError::PeriodNotElapsed
        );
    }

    // Initialize the contribution record if this is the first deposit for the period.
    if record.period_index == 0 && record.amount_deposited == 0 {
        record.affiliate    = ctx.accounts.affiliate.key();
        record.period_index = period_index;
        record.period_start = clock.unix_timestamp;
        record.met_minimum  = false;
        record.penalty_applied = false;
    }

    // Check and apply penalties for skipped periods.
    let expected_periods = if acc.last_contribution_at > 0 {
        ((clock.unix_timestamp - acc.enrolled_at) / cfg.contribution_period) as u32
    } else {
        0
    };
    let missed = expected_periods.saturating_sub(acc.periods_completed);
    if missed > cfg.grace_periods as u32 && !record.penalty_applied {
        // Apply penalty: reduce accrued yield by penalty_bps per missed period over grace.
        let penalty_periods = missed - cfg.grace_periods as u32;
        let total_penalty_bps = (cfg.missed_period_penalty_bps as u64)
            .checked_mul(penalty_periods as u64)
            .ok_or(PensionError::ContributionOverflow)?;
        let penalty = acc.accrued_yield
            .checked_mul(total_penalty_bps)
            .ok_or(PensionError::ContributionOverflow)?
            / 10_000;
        acc.accrued_yield = acc.accrued_yield.saturating_sub(penalty);
        acc.missed_periods = missed.min(255) as u8;
        record.penalty_applied = true;
        msg!("Penalty applied: {} tokens deducted from yield", penalty);
    }

    // Transfer tokens from affiliate to pool token account.
    let transfer_ctx = CpiContext::new(
          ctx.accounts.token_program.key(),
        Transfer {
            from:      ctx.accounts.affiliate_token_account.to_account_info(),
            to:        ctx.accounts.pool_token_account.to_account_info(),
            authority: ctx.accounts.affiliate.to_account_info(),
        },
    );
    token::transfer(transfer_ctx, amount)?;

    // Update record.
    record.amount_deposited = record.amount_deposited
        .checked_add(amount)
        .ok_or(PensionError::ContributionOverflow)?;
    record.last_deposit_at = clock.unix_timestamp;
    record.met_minimum = record.amount_deposited >= cfg.min_contribution;

    // Update affiliate account.
    acc.total_contributed = acc.total_contributed
        .checked_add(amount)
        .ok_or(PensionError::ContributionOverflow)?;
    acc.last_contribution_at = clock.unix_timestamp;
    if record.met_minimum {
        acc.periods_completed = acc.periods_completed.saturating_add(1);
        acc.missed_periods = 0;
    }

    // Update pool totals.
    pool.total_principal = pool.total_principal
        .checked_add(amount)
        .ok_or(PensionError::ContributionOverflow)?;

    msg!(
        "Contribution accepted: {} tokens | period {} | total contributed: {}",
        amount,
        period_index,
        acc.total_contributed
    );

    Ok(())
}