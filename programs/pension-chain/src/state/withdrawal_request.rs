use anchor_lang::prelude::*;

/// The reason a withdrawal was requested.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default)]
#[borsh(use_discriminant = true)]
pub enum WithdrawalReason {
    #[default]
    Retirement       = 0,
    MedicalEmergency = 1,
    NaturalDisaster  = 2,
    EarlyVoluntary   = 3,
    Inheritance      = 4,
}

/// Status of the withdrawal request lifecycle.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default)]
#[borsh(use_discriminant = true)]
pub enum WithdrawalStatus {
    #[default]
    Pending   = 0,
    Approved  = 1,
    Executed  = 2,
    Rejected  = 3,
}

/// A withdrawal request account — one per withdrawal event.
/// PDA seeds: [b"withdrawal", affiliate.key().as_ref(), &request_nonce.to_le_bytes()]
#[account]
#[derive(Default)]
pub struct WithdrawalRequest {
    /// The affiliate requesting the withdrawal.
    pub affiliate: Pubkey,

    /// Sequential nonce for this affiliate's withdrawals.
    pub request_nonce: u32,

    /// Why the withdrawal is being requested.
    pub reason: WithdrawalReason,

    /// Current lifecycle status.
    pub status: WithdrawalStatus,

    /// Amount requested in token base units.
    pub amount_requested: u64,

    /// Amount actually released (may differ due to penalties/caps).
    pub amount_released: u64,

    /// Penalty applied in token base units.
    pub penalty_amount: u64,

    /// Unix timestamp when the request was created.
    pub requested_at: i64,

    /// Unix timestamp when the request was approved/rejected.
    pub resolved_at: i64,

    /// For oracle-dependent withdrawals: hash of the oracle attestation document.
    pub oracle_attestation_hash: [u8; 32],

    /// For inheritance: the beneficiary wallet receiving this payout.
    pub beneficiary_wallet: Option<Pubkey>,

    /// Bump seed.
    pub bump: u8,
}

impl WithdrawalRequest {
    pub const LEN: usize = 8   // discriminator
        + 32   // affiliate
        + 4    // request_nonce
        + 1    // reason
        + 1    // status
        + 8    // amount_requested
        + 8    // amount_released
        + 8    // penalty_amount
        + 8    // requested_at
        + 8    // resolved_at
        + 32   // oracle_attestation_hash
        + 1 + 32  // Option<Pubkey>
        + 1;   // bump
}