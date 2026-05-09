// tests/test_02_affiliate_registration.rs

mod helpers;
use helpers::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::Signer,
    transaction::Transaction,
};

#[test]
fn register_affiliate_creates_account() {
    let mut env = TestEnv::new();
    let affiliate = solana_sdk::keypair::Keypair::new();

    let affiliate_account = env.register_affiliate(
        &affiliate,
        dob_for_age(35),
        1, // balanced
        vec![],
    );

    let account = env.svm.get_account(&affiliate_account);
    assert!(account.is_some(), "affiliate PDA should exist after registration");
    assert!(account.unwrap().data.len() > 8);
}

#[test]
fn register_affiliate_increments_pool_count() {
    let mut env = TestEnv::new();
    let (balanced_pool, _) = pool_pda(1);

    // Read affiliate_count before (fresh pool = 0 count in data, but we check
    // the account exists and grows — for full deserialization we'd use Anchor client).
    let pool_before = env.svm.get_account(&balanced_pool).unwrap();
    let count_offset = 8 + 1 + 32 + 8 + 8 + 2 + 8 + 4; // skip discriminator + fields to affiliate_count
    let count_before = u32::from_le_bytes(
        pool_before.data[count_offset..count_offset + 4]
            .try_into()
            .unwrap(),
    );

    let affiliate = solana_sdk::keypair::Keypair::new();
    env.register_affiliate(&affiliate, dob_for_age(30), 1, vec![]);

    let pool_after = env.svm.get_account(&balanced_pool).unwrap();
    let count_after = u32::from_le_bytes(
        pool_after.data[count_offset..count_offset + 4]
            .try_into()
            .unwrap(),
    );

    assert_eq!(count_after, count_before + 1, "pool affiliate_count should increment");
}

#[test]
fn register_with_valid_beneficiaries_succeeds() {
    let mut env = TestEnv::new();
    let affiliate = solana_sdk::keypair::Keypair::new();
    let ben1 = solana_sdk::keypair::Keypair::new();
    let ben2 = solana_sdk::keypair::Keypair::new();

    // Two beneficiaries summing to 10_000 bps
    let affiliate_account = env.register_affiliate(
        &affiliate,
        dob_for_age(40),
        0, // conservative
        vec![
            (ben1.pubkey(), 6_000),
            (ben2.pubkey(), 4_000),
        ],
    );

    assert!(env.svm.get_account(&affiliate_account).is_some());
}

#[test]
fn register_with_invalid_beneficiary_shares_fails() {
    let mut env = TestEnv::new();
    let affiliate = solana_sdk::keypair::Keypair::new();
    let ben = solana_sdk::keypair::Keypair::new();

    env.svm
        .airdrop(&affiliate.pubkey(), 10_000_000_000)
        .unwrap();

    let (affiliate_account, _) = affiliate_pda(&affiliate.pubkey());
    let (pool, _) = pool_pda(0);
    let discriminant = anchor_discriminant("register_affiliate");

    let mut data = discriminant.to_vec();
    data.extend_from_slice(&dob_for_age(30).to_le_bytes());
    data.extend_from_slice(&[72u8, 78u8]);
    data.extend_from_slice(&fake_hash());
    data.push(0u8); // conservative
    data.extend_from_slice(&fake_hash());
    // ONE beneficiary at only 5_000 bps (not 10_000) — should fail
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(ben.pubkey().as_ref());
    data.extend_from_slice(&5_000u16.to_le_bytes());

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(env.fund_config, false),
            AccountMeta::new(affiliate_account, false),
            AccountMeta::new(pool, false),
            AccountMeta::new(affiliate.pubkey(), true),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&affiliate.pubkey()),
        &[&affiliate],
        env.svm.latest_blockhash(),
    );

    let result = env.svm.send_transaction(tx);
    assert!(result.is_err(), "invalid beneficiary shares should fail");
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(
        err_str.contains("InvalidBeneficiaryShares") || err_str.contains("6005"),
        "expected InvalidBeneficiaryShares error, got: {err_str}"
    );
}

#[test]
fn affiliate_pdas_are_unique_per_wallet() {
    let kp1 = solana_sdk::keypair::Keypair::new();
    let kp2 = solana_sdk::keypair::Keypair::new();

    let (pda1, _) = affiliate_pda(&kp1.pubkey());
    let (pda2, _) = affiliate_pda(&kp2.pubkey());

    assert_ne!(pda1, pda2, "two different wallets must yield different PDAs");
}