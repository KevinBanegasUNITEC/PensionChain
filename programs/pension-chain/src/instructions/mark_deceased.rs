use anchor_lang::prelude::*;

use crate::errors::PensionError;
use crate::state::{AffiliateAccount, FundConfig};

#[derive(Accounts)]
pub struct MarkDeceased<'info> {
    #[account(
        seeds = [b"fund_config"],
        bump = fund_config.bump,
        constraint = fund_config.authority == oracle_authority.key() @ PensionError::InvalidOracleSigner,
    )]
    pub fund_config: Account<'info, FundConfig>,

    #[account(
        mut,
        seeds = [b"affiliate", affiliate_account.owner.as_ref()],
        bump = affiliate_account.bump,
        constraint = !affiliate_account.deceased @ PensionError::AccountDeceased,
    )]
    pub affiliate_account: Account<'info, AffiliateAccount>,

    /// The oracle authority that signs death attestations.
    pub oracle_authority: Signer<'info>,
}

pub fn handler(
    ctx: Context<MarkDeceased>,
    oracle_attestation_hash: [u8; 32],
) -> Result<()> {
    require!(
        oracle_attestation_hash != [0u8; 32],
        PensionError::MissingAttestation
    );

    require!(
        !ctx.accounts.affiliate_account.beneficiaries.is_empty(),
        PensionError::NoBeneficiaries
    );

    ctx.accounts.affiliate_account.deceased = true;

    msg!(
        "Affiliate {} marked as deceased | attestation: {:?}",
        ctx.accounts.affiliate_account.owner,
        &oracle_attestation_hash[..4]
    );

    Ok(())
}