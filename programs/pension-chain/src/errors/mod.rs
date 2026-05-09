use anchor_lang::prelude::*;

#[error_code]
pub enum PensionError {
    // ── Fund config ──────────────────────────────────────────────────────────
    #[msg("The fund is currently paused")]
    FundPaused,

    #[msg("New affiliate registrations are closed")]
    RegistrationsClosed,

    #[msg("Unauthorized: caller is not the fund authority")]
    Unauthorized,

    #[msg("Allocation basis points do not sum to 10,000")]
    InvalidAllocation,

    #[msg("Contribution period must be greater than zero")]
    InvalidContributionPeriod,

    #[msg("Minimum contribution must be greater than zero")]
    InvalidMinContribution,

    #[msg("Minimum retirement age is invalid")]
    InvalidRetirementAge,

    #[msg("Invalid risk profile")]
    InvalidRiskProfile,

    // ── Affiliate / KYC ─────────────────────────────────────────────────────
    #[msg("Affiliate is already registered")]
    AlreadyRegistered,

    #[msg("Beneficiary basis points do not sum to 10,000")]
    InvalidBeneficiaryShares,

    #[msg("Maximum number of beneficiaries (5) already reached")]
    TooManyBeneficiaries,

    #[msg("Affiliate account is marked as deceased")]
    AccountDeceased,

    // ── Contributions ────────────────────────────────────────────────────────
    #[msg("Contribution amount is below the minimum required")]
    BelowMinimumContribution,

    #[msg("Contribution period has not elapsed yet")]
    PeriodNotElapsed,

    #[msg("Arithmetic overflow in contribution calculation")]
    ContributionOverflow,

    // ── Withdrawals ─────────────────────────────────────────────────────────
    #[msg("Affiliate has not reached minimum retirement age")]
    NotRetirementAge,

    #[msg("Minimum contribution years not met")]
    InsufficientContributionYears,

    #[msg("Annual withdrawal cap exceeded")]
    AnnualCapExceeded,

    #[msg("Withdrawal amount exceeds available balance")]
    InsufficientFunds,

    #[msg("Withdrawal request is not in Pending status")]
    RequestNotPending,

    #[msg("Withdrawal request is not in Approved status")]
    RequestNotApproved,

    #[msg("Withdrawal mode has not been set")]
    WithdrawalModeNotSet,

    #[msg("Early voluntary withdrawal: affiliate is already eligible for retirement")]
    AlreadyEligibleForRetirement,

    // ── Oracle / attestations ────────────────────────────────────────────────
    #[msg("Oracle attestation hash is missing or zero")]
    MissingAttestation,

    #[msg("Oracle signer is not the trusted oracle authority")]
    InvalidOracleSigner,

    // ── Investment pool ──────────────────────────────────────────────────────
    #[msg("Pool token account does not match the registered pool")]
    PoolMismatch,

    #[msg("Harvest interval has not elapsed yet")]
    HarvestTooEarly,

    // ── Inheritance ──────────────────────────────────────────────────────────
    #[msg("No beneficiaries registered on this account")]
    NoBeneficiaries,

    #[msg("Beneficiary wallet does not match any registered beneficiary")]
    BeneficiaryNotFound,

    #[msg("Affiliate is not marked as deceased")]
    NotDeceased,
}