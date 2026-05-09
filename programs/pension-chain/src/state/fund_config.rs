// programs/pension-chain/src/state/fund_config.rs

use anchor_lang::prelude::*;

#[account]
pub struct FundConfig {
    pub authority: Pubkey,
    pub accepted_mint: Pubkey,
    pub treasury: Pubkey,

    pub min_contribution: u64,
    pub contribution_period: i64,

    pub grace_periods: u8,
    pub missed_period_penalty_bps: u16,

    pub min_retirement_age: u8,
    pub min_contribution_years: u8,

    pub early_withdrawal_penalty_bps: u16,
    pub max_annual_withdrawal_bps: u16,

    pub emergency_release_bps: u16,
    pub disaster_release_bps: u16,

    pub protocol_fee_bps: u16,

    pub registrations_open: bool,
    pub paused: bool,

    pub bump: u8,
}

impl FundConfig {
    pub const LEN: usize = 8 +
        32 +
        32 +
        32 +
        8 +
        8 +
        1 +
        2 +
        1 +
        1 +
        2 +
        2 +
        2 +
        2 +
        2 +
        1 +
        1 +
        1;
}