// tests/test_05_withdrawals.rs

mod helpers;
use helpers::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::Signer,
    transaction::Transaction,
};

fn setup_contributed(env: &mut TestEnv, age: i64, profile: u8) -> (
    solana_sdk::keypair::Keypair,
    solana_sdk::pubkey::Pubkey,
    solana_sdk::pubkey::Pubkey,
) {
    let affiliate = solana_sdk::keypair::Keypair::new();
    let affiliate_account =
        env.register_affiliate(&affiliate, dob_for_age(age), profile, vec![]);
    let token = create_token_account(&mut env.svm, &env.authority, &env.mint, &affiliate.pubkey());
    mint_tokens(&mut env.svm, &env.authority, &env.mint, &token, 10_000_000);
    env.contribute(&affiliate, token, profile, 0, MIN_CONTRIBUTION);
    let (pool, _) = pool_pda(profile);
    (affiliate, affiliate_account, pool)
}

// ── Retirement ────────────────────────────────────────────────────────────────

#[test]
fn retirement_withdrawal_below_age_fails() {
    let mut env = TestEnv::new();
    let (affiliate, affiliate_account, pool) = setup_contributed(&mut env, 30, 1);
    let (req, _) = withdrawal_request_pda(&affiliate.pubkey(), 0);

    let discriminant = anchor_discriminant("request_retirement_withdrawal");
    let mut data = discriminant.to_vec();
    data.extend_from_slice(&0u32.to_le_bytes()); // nonce
    data.extend_from_slice(&MIN_CONTRIBUTION.to_le_bytes()); // amount
    data.push(3u8); // WithdrawalMode::LumpSum = 3

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(env.fund_config, false),
            AccountMeta::new(affiliate_account, false),
            AccountMeta::new_readonly(pool, false),
            AccountMeta::new(req, false),
            AccountMeta::new(affiliate.pubkey(), true),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&affiliate.pubkey()), &[&affiliate], env.svm.latest_blockhash(),
    );
    let result = env.svm.send_transaction(tx);
    assert!(result.is_err(), "should fail: age 30 < 65");
    let err = format!("{:?}", result.unwrap_err());
    assert!(
        err.contains("NotRetirementAge") || err.contains("5002"),
        "expected NotRetirementAge, got: {err}"
    );
}

#[test]
fn retirement_withdrawal_with_not_set_mode_fails() {
    let mut env = TestEnv::new();
    let (affiliate, affiliate_account, pool) = setup_contributed(&mut env, 70, 1);
    let (req, _) = withdrawal_request_pda(&affiliate.pubkey(), 0);

    let discriminant = anchor_discriminant("request_retirement_withdrawal");
    let mut data = discriminant.to_vec();
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&MIN_CONTRIBUTION.to_le_bytes());
    data.push(0u8); // WithdrawalMode::NotSet = 0

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(env.fund_config, false),
            AccountMeta::new(affiliate_account, false),
            AccountMeta::new_readonly(pool, false),
            AccountMeta::new(req, false),
            AccountMeta::new(affiliate.pubkey(), true),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&affiliate.pubkey()), &[&affiliate], env.svm.latest_blockhash(),
    );
    let result = env.svm.send_transaction(tx);
    assert!(result.is_err(), "should fail: mode NotSet");
    let err = format!("{:?}", result.unwrap_err());
    assert!(
        err.contains("WithdrawalModeNotSet") || err.contains("5006"),
        "expected WithdrawalModeNotSet, got: {err}"
    );
}

// ── Emergency ─────────────────────────────────────────────────────────────────

#[test]
fn emergency_withdrawal_with_valid_attestation_succeeds() {
    let mut env = TestEnv::new();
    let (affiliate, affiliate_account, _pool) = setup_contributed(&mut env, 45, 1);
    let (req, _) = withdrawal_request_pda(&affiliate.pubkey(), 0);

    let discriminant = anchor_discriminant("request_emergency_withdrawal");
    let mut data = discriminant.to_vec();
    data.extend_from_slice(&0u32.to_le_bytes()); // nonce
    data.push(1u8); // WithdrawalReason::MedicalEmergency = 1
    data.extend_from_slice(&fake_hash()); // oracle attestation

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(env.fund_config, false),
            AccountMeta::new(affiliate_account, false),
            AccountMeta::new(req, false),
            AccountMeta::new(affiliate.pubkey(), true),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&affiliate.pubkey()), &[&affiliate], env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx).expect("emergency withdrawal should succeed");

    assert!(
        env.svm.get_account(&req).is_some(),
        "withdrawal request PDA should be created"
    );
}

#[test]
fn emergency_withdrawal_with_zero_attestation_fails() {
    let mut env = TestEnv::new();
    let (affiliate, affiliate_account, _) = setup_contributed(&mut env, 45, 1);
    let (req, _) = withdrawal_request_pda(&affiliate.pubkey(), 0);

    let discriminant = anchor_discriminant("request_emergency_withdrawal");
    let mut data = discriminant.to_vec();
    data.extend_from_slice(&0u32.to_le_bytes());
    data.push(1u8); // MedicalEmergency
    data.extend_from_slice(&zero_hash()); // zero hash — should fail

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(env.fund_config, false),
            AccountMeta::new(affiliate_account, false),
            AccountMeta::new(req, false),
            AccountMeta::new(affiliate.pubkey(), true),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&affiliate.pubkey()), &[&affiliate], env.svm.latest_blockhash(),
    );
    let result = env.svm.send_transaction(tx);
    assert!(result.is_err(), "zero attestation must fail");
    let err = format!("{:?}", result.unwrap_err());
    assert!(
        err.contains("MissingAttestation") || err.contains("6000"),
        "expected MissingAttestation, got: {err}"
    );
}

// ── Early voluntary ──────────────────────────────────────────────────────────

#[test]
fn early_voluntary_withdrawal_creates_request() {
    let mut env = TestEnv::new();
    let (affiliate, affiliate_account, _) = setup_contributed(&mut env, 30, 1);
    let (req, _) = withdrawal_request_pda(&affiliate.pubkey(), 0);

    let discriminant = anchor_discriminant("request_early_voluntary_withdrawal");
    let mut data = discriminant.to_vec();
    data.extend_from_slice(&0u32.to_le_bytes()); // nonce

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(env.fund_config, false),
            AccountMeta::new(affiliate_account, false),
            AccountMeta::new(req, false),
            AccountMeta::new(affiliate.pubkey(), true),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&affiliate.pubkey()), &[&affiliate], env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx).expect("early voluntary withdrawal");
    assert!(env.svm.get_account(&req).is_some());
}

#[test]
fn withdrawal_request_pdas_are_sequential_and_unique() {
    let kp = solana_sdk::keypair::Keypair::new();
    let (req0, _) = withdrawal_request_pda(&kp.pubkey(), 0);
    let (req1, _) = withdrawal_request_pda(&kp.pubkey(), 1);
    assert_ne!(req0, req1, "different nonces must give different PDAs");
}