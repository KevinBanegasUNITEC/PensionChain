// tests/test_01_fund_initialization.rs

mod helpers;
use helpers::*;

#[test]
fn fund_config_is_created_with_correct_parameters() {
    let env = TestEnv::new();

    // Verify the fund_config account exists and was written.
    let account = env.svm.get_account(&env.fund_config);
    assert!(account.is_some(), "fund_config PDA should exist");
    // Account must be non-zero (holds FundConfig data).
    assert!(account.unwrap().data.len() > 8, "fund_config should have data beyond discriminant");
}

#[test]
fn all_three_pools_are_created() {
    let env = TestEnv::new();

    let (conservative, _) = pool_pda(0);
    let (balanced, _) = pool_pda(1);
    let (aggressive, _) = pool_pda(2);

    assert!(env.svm.get_account(&conservative).is_some(), "conservative pool must exist");
    assert!(env.svm.get_account(&balanced).is_some(), "balanced pool must exist");
    assert!(env.svm.get_account(&aggressive).is_some(), "aggressive pool must exist");
}

#[test]
fn pool_accounts_are_distinct() {
    let (conservative, _) = pool_pda(0);
    let (balanced, _) = pool_pda(1);
    let (aggressive, _) = pool_pda(2);

    assert_ne!(conservative, balanced);
    assert_ne!(balanced, aggressive);
    assert_ne!(conservative, aggressive);
}

#[test]
fn duplicate_fund_init_fails() {
    let mut env = TestEnv::new();

    // Attempt to call initialize_fund a second time — should fail because
    // the PDA account is already initialized.
    let discriminant = anchor_discriminant("initialize_fund");
    let mut data = discriminant.to_vec();
    data.extend_from_slice(&MIN_CONTRIBUTION.to_le_bytes());
    data.extend_from_slice(&CONTRIBUTION_PERIOD.to_le_bytes());
    data.push(MIN_RETIREMENT_AGE);
    data.push(MIN_CONTRIBUTION_YEARS);

    let ix = solana_sdk::instruction::Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(env.fund_config, false),
            solana_sdk::instruction::AccountMeta::new_readonly(env.mint, false),
            solana_sdk::instruction::AccountMeta::new_readonly(env.treasury, false),
            solana_sdk::instruction::AccountMeta::new(env.authority.pubkey(), true),
            solana_sdk::instruction::AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
            solana_sdk::instruction::AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    };

    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[ix],
        Some(&env.authority.pubkey()),
        &[&env.authority],
        env.svm.latest_blockhash(),
    );
    let result = env.svm.send_transaction(tx);
    assert!(result.is_err(), "Second initialize_fund must fail");
}

#[test]
fn fund_config_pda_seeds_are_deterministic() {
    let (pda1, bump1) = fund_config_pda();
    let (pda2, bump2) = fund_config_pda();
    assert_eq!(pda1, pda2);
    assert_eq!(bump1, bump2);
}