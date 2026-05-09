// programs/pension-chain/src/instructions/initialize_fund.rs

use anchor_lang::prelude::*;

use anchor_spl::associated_token::AssociatedToken;

use anchor_spl::token::{
    Mint,
    Token,
    TokenAccount,
};

use crate::errors::PensionError;
use crate::state::FundConfig;

#[derive(Accounts)]
pub struct InitializeFund<'info> {
    #[account(
        init,
        payer = authority,
        space = FundConfig::LEN,
        seeds = [b"fund_config"],
        bump
    )]
    pub fund_config: Account<'info, FundConfig>,

    pub accepted_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = accepted_mint,
        associated_token::authority = fund_config,
    )]
    pub treasury: Account<'info, TokenAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program:
        Program<'info, AssociatedToken>,

    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<InitializeFund>,
    min_contribution: u64,
    contribution_period: i64,
    min_retirement_age: u8,
    min_contribution_years: u8,
) -> Result<()> {

    require!(
        contribution_period > 0,
        PensionError::InvalidContributionPeriod
    );

    require!(
        min_contribution > 0,
        PensionError::InvalidMinContribution
    );

    require!(
        min_retirement_age >= 40,
        PensionError::InvalidRetirementAge
    );

    let cfg = &mut ctx.accounts.fund_config;

    cfg.authority = ctx.accounts.authority.key();

    cfg.accepted_mint =
        ctx.accounts.accepted_mint.key();

    cfg.treasury =
        ctx.accounts.treasury.key();

    cfg.min_contribution =
        min_contribution;

    cfg.contribution_period =
        contribution_period;

    cfg.grace_periods = 3;

    cfg.missed_period_penalty_bps = 100;

    cfg.min_retirement_age =
        min_retirement_age;

    cfg.min_contribution_years =
        min_contribution_years;

    cfg.early_withdrawal_penalty_bps = 1_000;

    cfg.max_annual_withdrawal_bps = 1_500;

    cfg.emergency_release_bps = 3_000;

    cfg.disaster_release_bps = 3_000;

    cfg.protocol_fee_bps = 50;

    cfg.registrations_open = true;

    cfg.paused = false;

    cfg.bump = ctx.bumps.fund_config;

    msg!("Pension fund initialized");

    Ok(())
}