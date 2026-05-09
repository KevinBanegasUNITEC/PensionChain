use anchor_lang::prelude::*;

/// Ledger entry for a single contribution period.
/// PDA seeds: [b"contribution", affiliate.key().as_ref(), &period_index.to_le_bytes()]
///
/// Kept lightweight — we don't store one per tx but one per period so that
/// the keeper and withdrawal controller can verify history without scanning
/// all transactions.
#[account]
#[derive(Default)]
pub struct ContributionRecord {
    /// The affiliate this record belongs to.
    pub affiliate: Pubkey,

    /// Sequential period index (0-based). One per contribution_period interval.
    pub period_index: u32,

    /// Total amount deposited during this period (may be multiple deposits).
    pub amount_deposited: u64,

    /// Unix timestamp of the first deposit in this period.
    pub period_start: i64,

    /// Unix timestamp of the last deposit in this period.
    pub last_deposit_at: i64,

    /// Whether the minimum contribution requirement was met this period.
    pub met_minimum: bool,

    /// Whether a missed-period penalty was applied.
    pub penalty_applied: bool,

    /// Bump seed.
    pub bump: u8,
}

impl ContributionRecord {
    pub const LEN: usize = 8   // discriminator
        + 32   // affiliate
        + 4    // period_index
        + 8    // amount_deposited
        + 8    // period_start
        + 8    // last_deposit_at
        + 1    // met_minimum
        + 1    // penalty_applied
        + 1;   // bump
}