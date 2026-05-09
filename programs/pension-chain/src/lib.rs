use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;
use state::{Beneficiary, RiskProfile, WithdrawalMode, WithdrawalReason};

declare_id!("Azxg2uCjx4oJNBgjQ4ZosnjVy38wNgntmXnh8seTXwhj");

#[program]
pub mod pension_chain {
    use super::*;

    // ── Fund administration ──────────────────────────────────────────────────

    /// Create the global fund configuration. Called once by the deployer.
    pub fn initialize_fund(
        ctx: Context<InitializeFund>,
        min_contribution: u64,
        contribution_period: i64,
        min_retirement_age: u8,
        min_contribution_years: u8,
    ) -> Result<()> {
        initialize_fund::handler(
            ctx,
            min_contribution,
            contribution_period,
            min_retirement_age,
            min_contribution_years,
        )
    }

    /// Create one investment pool per risk profile (called 3 times by admin).
    pub fn initialize_pool(ctx: Context<InitializePool>, risk_profile: RiskProfile) -> Result<()> {
        initialize_pool::handler(ctx, risk_profile)
    }

    // ── Affiliate lifecycle ──────────────────────────────────────────────────

    /// Register a new affiliate. Includes KYC hash, DOB, risk profile, and beneficiaries.
    pub fn register_affiliate(
        ctx: Context<RegisterAffiliate>,
        date_of_birth: i64,
        country_code: [u8; 2],
        kyc_doc_hash: [u8; 32],
        risk_profile: RiskProfile,
        advisor_session_hash: [u8; 32],
        beneficiaries: Vec<Beneficiary>,
    ) -> Result<()> {
        register_affiliate::handler(
            ctx,
            date_of_birth,
            country_code,
            kyc_doc_hash,
            risk_profile,
            advisor_session_hash,
            beneficiaries,
        )
    }

    /// Change the affiliate's risk profile (max once per 6 months).
    pub fn update_risk_profile(
        ctx: Context<UpdateRiskProfile>,
        new_risk_profile: RiskProfile,
        advisor_session_hash: [u8; 32],
    ) -> Result<()> {
        update_risk_profile::handler(ctx, new_risk_profile, advisor_session_hash)
    }

    /// Oracle-attested death marking — enables inheritance processing.
    pub fn mark_deceased(
        ctx: Context<MarkDeceased>,
        oracle_attestation_hash: [u8; 32],
    ) -> Result<()> {
        mark_deceased::handler(ctx, oracle_attestation_hash)
    }

    // ── Contributions ────────────────────────────────────────────────────────

    /// Deposit tokens for a given contribution period.
    pub fn contribute(
        ctx: Context<Contribute>,
        period_index: u32,
        amount: u64,
    ) -> Result<()> {
        contribute::handler(ctx, period_index, amount)
    }

    // ── Yield management (keeper) ────────────────────────────────────────────

    /// Keeper reports new APY and credits harvested yield to an affiliate.
    pub fn harvest_yield(
        ctx: Context<HarvestYield>,
        new_apy_bps: u16,
        yield_amount: u64,
    ) -> Result<()> {
        harvest_yield::handler(ctx, new_apy_bps, yield_amount)
    }

    // ── Withdrawals ──────────────────────────────────────────────────────────

    /// Standard retirement withdrawal request.
    pub fn request_retirement_withdrawal(
        ctx: Context<RequestRetirementWithdrawal>,
        request_nonce: u32,
        amount_requested: u64,
        withdrawal_mode: WithdrawalMode,
    ) -> Result<()> {
        request_retirement_withdrawal::handler(ctx, request_nonce, amount_requested, withdrawal_mode)
    }

    /// Emergency withdrawal request (medical or disaster) — requires oracle attestation.
    pub fn request_emergency_withdrawal(
        ctx: Context<RequestEmergencyWithdrawal>,
        request_nonce: u32,
        reason: WithdrawalReason,
        oracle_attestation_hash: [u8; 32],
    ) -> Result<()> {
        request_emergency_withdrawal::handler(ctx, request_nonce, reason, oracle_attestation_hash)
    }

    /// Early voluntary withdrawal — 10% of balance with yield penalty.
    pub fn request_early_voluntary_withdrawal(
        ctx: Context<RequestEarlyVoluntaryWithdrawal>,
        request_nonce: u32,
    ) -> Result<()> {
        request_early_voluntary_withdrawal::handler(ctx, request_nonce)
    }

    /// Keeper approves a pending withdrawal request and executes the token transfer.
    pub fn approve_and_execute_withdrawal(
        ctx: Context<ApproveAndExecuteWithdrawal>,
    ) -> Result<()> {
        approve_and_execute_withdrawal::handler(ctx)
    }

    // ── Inheritance ──────────────────────────────────────────────────────────

    /// Process a single beneficiary's inheritance payout after death is confirmed.
    pub fn process_inheritance(
        ctx: Context<ProcessInheritance>,
        request_nonce: u32,
        beneficiary_index: u8,
        oracle_attestation_hash: [u8; 32],
    ) -> Result<()> {
        process_inheritance::handler(ctx, request_nonce, beneficiary_index, oracle_attestation_hash)
    }
}