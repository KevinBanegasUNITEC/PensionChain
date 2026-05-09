use anchor_lang::prelude::*;
use crate::state::affiliate::RiskProfile;

/// On-chain record for each investment pool (Conservative / Balanced / Aggressive).
/// PDA seeds: [b"pool", &[risk_profile as u8]]
#[account]
#[derive(Default)]
pub struct InvestmentPool {
    /// Which risk profile this pool serves.
    pub risk_profile: RiskProfile,

    /// Token account that holds this pool's liquid balance (USDC).
    pub token_account: Pubkey,

    /// Total principal currently held in the pool (base units).
    pub total_principal: u64,

    /// Total yield harvested and not yet distributed (base units).
    pub total_yield: u64,

    /// Current APY in basis points as reported by the last keeper harvest.
    /// e.g. 800 = 8.00%
    pub current_apy_bps: u16,

    /// Unix timestamp of the last yield harvest by the keeper.
    pub last_harvest_at: i64,

    /// Number of active affiliates currently allocated to this pool.
    pub affiliate_count: u32,

    /// Allocation breakdown (bps) — must sum to 10_000.
    /// Index 0: stablecoins / money market
    /// Index 1: tokenized bonds / RWA
    /// Index 2: tokenized equities
    /// Index 3: DeFi protocols (Aave, Curve, etc.)
    /// Index 4: high-risk / growth
    pub allocation_bps: [u16; 5],

    /// Bump seed.
    pub bump: u8,
}

impl InvestmentPool {
    pub const LEN: usize = 8   // discriminator
        + 1    // risk_profile
        + 32   // token_account
        + 8    // total_principal
        + 8    // total_yield
        + 2    // current_apy_bps
        + 8    // last_harvest_at
        + 4    // affiliate_count
        + 10   // allocation_bps [u16; 5]
        + 1;   // bump

    /// Default allocations per profile.
    pub fn default_allocation(profile: RiskProfile) -> [u16; 5] {
        match profile {
            // Conservative: 40% stables, 40% bonds, 20% equities, 0 DeFi, 0 high-risk
            RiskProfile::Conservative => [4_000, 4_000, 2_000, 0, 0],
            // Balanced: 20% stables, 30% bonds, 30% equities, 20% DeFi, 0 high-risk
            RiskProfile::Balanced     => [2_000, 3_000, 3_000, 2_000, 0],
            // Aggressive: 5% stables, 10% bonds, 25% equities, 35% DeFi, 25% high-risk
            RiskProfile::Aggressive   => [500, 1_000, 2_500, 3_500, 2_500],
        }
    }

    /// Validate that allocation percentages sum to exactly 10_000.
    pub fn allocation_valid(&self) -> bool {
        let total: u32 = self.allocation_bps.iter().map(|&x| x as u32).sum();
        total == 10_000
    }
}