// tests/test_03_contributions.rs

mod helpers;
use helpers::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::Signer,
    transaction::Transaction,
};

fn setup_affiliate_with_tokens(
    env: &mut TestEnv,
    profile_index: u8,
) -> (solana_sdk::keypair::Keypair, solana_sdk::pubkey::Pubkey) {
    let affiliate = solana_sdk::keypair::Keypair::new();
    env.register_affiliate(&affiliate, dob_for_age(35), profile_index, vec![]);

    let affiliate_token = create_token_account(
        &mut env.svm,
        &env.authority,
        &env.mint,
        &affiliate.pubkey(),
    );
    mint_tokens(
        &mut env.svm,
        &env.authority,
        &env.mint,
        &affiliate_token,
        10_000_000,
    ); // 10 USDC
    (affiliate, affiliate_token)
}

#[test]
fn valid_contribution_transfers_tokens_to_pool() {
    let mut env = TestEnv::new();
    let (affiliate, affiliate_token) = setup_affiliate_with_tokens(&mut env, 1);

    let before = token_balance(&env.svm, &env.balanced_pool_token);
    env.contribute(&affiliate, affiliate_token, 1, 0, MIN_CONTRIBUTION);
    let after = token_balance(&env.svm, &env.balanced_pool_token);

    assert_eq!(after - before, MIN_CONTRIBUTION, "pool should receive exactly MIN_CONTRIBUTION tokens");
}

#[test]
fn contribution_creates_record_account() {
    let mut env = TestEnv::new();
    let (affiliate, affiliate_token) = setup_affiliate_with_tokens(&mut env, 0);

    let (record, _) = contribution_record_pda(&affiliate.pubkey(), 0);
    assert!(
        env.svm.get_account(&record).is_none(),
        "record should not exist before contribution"
    );

    env.contribute(&affiliate, affiliate_token, 0, 0, MIN_CONTRIBUTION);

    assert!(
        env.svm.get_account(&record).is_some(),
        "contribution record PDA should exist after contribution"
    );
}

#[test]
fn contribution_below_minimum_fails() {
    let mut env = TestEnv::new();
    let (affiliate, affiliate_token) = setup_affiliate_with_tokens(&mut env, 1);

    let (affiliate_account, _) = affiliate_pda(&affiliate.pubkey());
    let (pool, _) = pool_pda(1);
    let (record, _) = contribution_record_pda(&affiliate.pubkey(), 0);

    let discriminant = anchor_discriminant("contribute");
    let mut data = discriminant.to_vec();
    data.extend_from_slice(&0u32.to_le_bytes()); // period_index = 0
    data.extend_from_slice(&100u64.to_le_bytes()); // amount = 100 (below 1_000_000 min)

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(env.fund_config, false),
            AccountMeta::new(affiliate_account, false),
            AccountMeta::new(pool, false),
            AccountMeta::new(record, false),
            AccountMeta::new(affiliate_token, false),
            AccountMeta::new(env.balanced_pool_token, false),
            AccountMeta::new(affiliate.pubkey(), true),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
            AccountMeta::new_readonly(spl_token::id(), false),
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
    assert!(result.is_err(), "contribution below minimum should fail");
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(
        err_str.contains("BelowMinimumContribution") || err_str.contains("4002"),
        "expected BelowMinimumContribution, got: {err_str}"
    );
}

#[test]
fn affiliate_token_balance_decreases_after_contribution() {
    let mut env = TestEnv::new();
    let (affiliate, affiliate_token) = setup_affiliate_with_tokens(&mut env, 2);

    let before = token_balance(&env.svm, &affiliate_token);
    env.contribute(&affiliate, affiliate_token, 2, 0, MIN_CONTRIBUTION);
    let after = token_balance(&env.svm, &affiliate_token);

    assert_eq!(
        before - after,
        MIN_CONTRIBUTION,
        "affiliate token balance should decrease by contribution amount"
    );
}

#[test]
fn contribution_record_pda_is_deterministic() {
    let kp = solana_sdk::keypair::Keypair::new();
    let (r1, b1) = contribution_record_pda(&kp.pubkey(), 0);
    let (r2, b2) = contribution_record_pda(&kp.pubkey(), 0);
    assert_eq!(r1, r2);
    assert_eq!(b1, b2);

    let (r3, _) = contribution_record_pda(&kp.pubkey(), 1);
    assert_ne!(r1, r3, "different periods must give different PDAs");
}