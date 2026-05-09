use anchor_lang::prelude::*;

/// Risk profile chosen (or recommended) for an affiliate.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default)]
#[borsh(use_discriminant = true)]
pub enum RiskProfile {
    #[default]
    Conservative = 0,
    Balanced     = 1,
    Aggressive   = 2,
}

/// Withdrawal modality selected at retirement.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default)]
#[borsh(use_discriminant = true)]
pub enum WithdrawalMode {
    #[default]
    NotSet           = 0,
    MonthlyPayments  = 1,
    ProgrammedFund   = 2,
    LumpSum          = 3,
    Mixed            = 4,
}

/// A registered beneficiary entry — wallet + percentage share.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct Beneficiary {
    pub wallet: Pubkey,
    /// Share in basis points. All beneficiary shares must sum to 10_000.
    pub share_bps: u16,
}

impl Beneficiary {
    pub const LEN: usize = 32 + 2; // wallet + share_bps
}

/// Core per-affiliate on-chain account.
/// PDA seeds: [b"affiliate", affiliate.key().as_ref()]
#[account]
#[derive(Default)]
pub struct AffiliateAccount {
    /// The wallet that owns this pension account.
    pub owner: Pubkey,

    /// Unix timestamp of the first contribution.
    pub enrolled_at: i64,

    /// Date of birth stored as Unix timestamp (midnight UTC of birth date).
    /// Set at registration via oracle-attested KYC hash verification.
    pub date_of_birth: i64,

    /// ISO 3166-1 alpha-2 country code stored as two ASCII bytes.
    pub country_code: [u8; 2],

    /// Hash of off-chain KYC document (stored on IPFS/Arweave).
    pub kyc_doc_hash: [u8; 32],

    /// Selected risk profile.
    pub risk_profile: RiskProfile,

    /// IPFS CID (32-byte hash) of the AI advisor session that produced
    /// the risk profile recommendation — audit trail.
    pub advisor_session_hash: [u8; 32],

    /// Total tokens contributed (principal only, no yield).
    pub total_contributed: u64,

    /// Accrued yield in token base units (updated by keeper on harvest).
    pub accrued_yield: u64,

    /// Timestamp of the most recent successful contribution.
    pub last_contribution_at: i64,

    /// Number of consecutive missed contribution periods.
    pub missed_periods: u8,

    /// Total number of contribution periods completed.
    pub periods_completed: u32,

    /// Whether the affiliate has reached retirement and unlocked withdrawals.
    pub retired: bool,

    /// Chosen withdrawal modality (set at retirement time).
    pub withdrawal_mode: WithdrawalMode,

    /// Amount withdrawn in the current calendar year (for 15% annual cap).
    pub withdrawn_this_year: u64,

    /// Calendar year of the `withdrawn_this_year` counter (reset on new year).
    pub withdrawal_year: i16,

    /// Registered beneficiaries (max 5).
    pub beneficiaries: Vec<Beneficiary>,

    /// Whether the account has been marked deceased (oracle-attested).
    pub deceased: bool,

    /// Bump seed for the PDA.
    pub bump: u8,
}

impl AffiliateAccount {
    /// Max beneficiaries per account.
    pub const MAX_BENEFICIARIES: usize = 5;

    pub const LEN: usize = 8   // discriminator
        + 32   // owner
        + 8    // enrolled_at
        + 8    // date_of_birth
        + 2    // country_code
        + 32   // kyc_doc_hash
        + 1    // risk_profile
        + 32   // advisor_session_hash
        + 8    // total_contributed
        + 8    // accrued_yield
        + 8    // last_contribution_at
        + 1    // missed_periods
        + 4    // periods_completed
        + 1    // retired
        + 1    // withdrawal_mode
        + 8    // withdrawn_this_year
        + 2    // withdrawal_year
        + 4 + (Self::MAX_BENEFICIARIES * Beneficiary::LEN)  // beneficiaries vec
        + 1    // deceased
        + 1;   // bump

    /// Age in full years as of the given unix timestamp.
    pub fn age_at(&self, ts: i64) -> u8 {
        let seconds_per_year: i64 = 365 * 24 * 3600;
        let years = (ts - self.date_of_birth) / seconds_per_year;
        years.clamp(0, 255) as u8
    }

    /// Years of contributions completed.
    pub fn contribution_years(&self) -> u32 {
        self.periods_completed / 12
    }

    /// Validate that beneficiary shares sum to exactly 10_000 bps.
    pub fn beneficiaries_valid(&self) -> bool {
        if self.beneficiaries.is_empty() {
            return true;
        }
        let total: u32 = self.beneficiaries.iter().map(|b| b.share_bps as u32).sum();
        total == 10_000
    }
}