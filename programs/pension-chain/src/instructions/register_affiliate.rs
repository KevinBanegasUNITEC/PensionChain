use anchor_lang::prelude::*;

use crate::errors::PensionError;
use crate::state::{AffiliateAccount, Beneficiary, FundConfig, InvestmentPool, RiskProfile};

#[derive(Accounts)]
#[instruction(
    date_of_birth: i64,
    country_code: [u8; 2],
    kyc_doc_hash: [u8; 32],
    risk_profile: RiskProfile,
    advisor_session_hash: [u8; 32],
    beneficiaries: Vec<Beneficiary>
)]
pub struct RegisterAffiliate<'info> {
    #[account(
        seeds = [b"fund_config"],
        bump = fund_config.bump,
        constraint = !fund_config.paused @ PensionError::FundPaused,
        constraint = fund_config.registrations_open @ PensionError::RegistrationsClosed,
    )]
    pub fund_config: Account<'info, FundConfig>,

    #[account(
        init,
        payer = affiliate,
        space = AffiliateAccount::LEN,
        seeds = [b"affiliate", affiliate.key().as_ref()],
        bump
    )]
    pub affiliate_account: Account<'info, AffiliateAccount>,

    /// The investment pool matching the chosen risk profile.
    #[account(
        mut,
        seeds = [b"pool", &[risk_profile as u8]],
        bump = pool.bump,
    )]
    pub pool: Account<'info, InvestmentPool>,

    #[account(mut)]
    pub affiliate: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<RegisterAffiliate>,
    date_of_birth: i64,
    country_code: [u8; 2],
    kyc_doc_hash: [u8; 32],
    risk_profile: RiskProfile,
    advisor_session_hash: [u8; 32],
    beneficiaries: Vec<Beneficiary>,
) -> Result<()> {
    // Validate beneficiaries if provided
    if !beneficiaries.is_empty() {
        require!(
            beneficiaries.len() <= AffiliateAccount::MAX_BENEFICIARIES,
            PensionError::TooManyBeneficiaries
        );
        let total: u32 = beneficiaries.iter().map(|b| b.share_bps as u32).sum();
        require!(total == 10_000, PensionError::InvalidBeneficiaryShares);
    }

    let clock = Clock::get()?;

    let acc = &mut ctx.accounts.affiliate_account;
    acc.owner                 = ctx.accounts.affiliate.key();
    acc.enrolled_at           = clock.unix_timestamp;
    acc.date_of_birth         = date_of_birth;
    acc.country_code          = country_code;
    acc.kyc_doc_hash          = kyc_doc_hash;
    acc.risk_profile          = risk_profile;
    acc.advisor_session_hash  = advisor_session_hash;
    acc.total_contributed     = 0;
    acc.accrued_yield         = 0;
    acc.last_contribution_at  = 0;
    acc.missed_periods        = 0;
    acc.periods_completed     = 0;
    acc.retired               = false;
    acc.withdrawal_mode       = crate::state::WithdrawalMode::NotSet;
    acc.withdrawn_this_year   = 0;
    acc.withdrawal_year       = 0;
    acc.beneficiaries         = beneficiaries;
    acc.deceased              = false;
    acc.bump                  = ctx.bumps.affiliate_account;

    // Increment pool affiliate count
    ctx.accounts.pool.affiliate_count = ctx.accounts.pool.affiliate_count
        .checked_add(1)
        .ok_or(PensionError::ContributionOverflow)?;

    msg!(
        "Affiliate registered: {} with {:?} profile",
        acc.owner,
        risk_profile as u8
    );

    Ok(())
}