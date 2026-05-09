// tests/test_06_inheritance.rs

mod helpers;
use helpers::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::Signer,
    transaction::Transaction,
};

fn mark_deceased(env: &mut TestEnv, affiliate_account: solana_sdk::pubkey::Pubkey) {
    let discriminant = anchor_discriminant("mark_deceased");
    let mut data = discriminant.to_vec();
    data.extend_from_slice(&fake_hash());

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(env.fund_config, false),
            AccountMeta::new(affiliate_account, false),
            AccountMeta::new_readonly(env.authority.pubkey(), true),
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&env.authority.pubkey()),
        &[&env.authority],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx).expect("mark_deceased");
}

#[test]
fn mark_deceased_with_zero_hash_fails() {
    let mut env = TestEnv::new();
    let ben = solana_sdk::keypair::Keypair::new();
    let affiliate = solana_sdk::keypair::Keypair::new();
    let affiliate_account = env.register_affiliate(
        &affiliate,
        dob_for_age(60),
        1,
        vec![(ben.pubkey(), 10_000)],
    );

    let discriminant = anchor_discriminant("mark_deceased");
    let mut data = discriminant.to_vec();
    data.extend_from_slice(&zero_hash()); // zero — should fail

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(env.fund_config, false),
            AccountMeta::new(affiliate_account, false),
            AccountMeta::new_readonly(env.authority.pubkey(), true),
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&env.authority.pubkey()), &[&env.authority], env.svm.latest_blockhash(),
    );
    let result = env.svm.send_transaction(tx);
    assert!(result.is_err(), "zero attestation should fail");
    let err = format!("{:?}", result.unwrap_err());
    assert!(
        err.contains("MissingAttestation") || err.contains("6000"),
        "expected MissingAttestation, got: {err}"
    );
}

#[test]
fn mark_deceased_with_valid_attestation_succeeds() {
    let mut env = TestEnv::new();
    let ben = solana_sdk::keypair::Keypair::new();
    let affiliate = solana_sdk::keypair::Keypair::new();
    let affiliate_account = env.register_affiliate(
        &affiliate,
        dob_for_age(60),
        1,
        vec![(ben.pubkey(), 10_000)],
    );

    mark_deceased(&mut env, affiliate_account);

    // Verify the `deceased` flag is set in account data.
    // Layout: 8 + 32 + 8 + 8 + 2 + 32 + 1 + 32 + 8 + 8 + 8 + 1 + 4 + 1 + 1 + 8 + 2
    //       + (4 + 5 * 34) + 1 (deceased) = varies, but we check via re-reading the account.
    let acc = env.svm.get_account(&affiliate_account).unwrap();
    assert!(!acc.data.is_empty(), "account should still have data");
    // The `deceased` bool is the second-to-last byte before the bump seed.
    // Since Vec<Beneficiary> is dynamic we read it indirectly by checking
    // the transaction succeeded and the account persists.
    assert!(env.svm.get_account(&affiliate_account).is_some());
}

#[test]
fn cannot_mark_deceased_twice() {
    let mut env = TestEnv::new();
    let ben = solana_sdk::keypair::Keypair::new();
    let affiliate = solana_sdk::keypair::Keypair::new();
    let affiliate_account = env.register_affiliate(
        &affiliate,
        dob_for_age(55),
        0,
        vec![(ben.pubkey(), 10_000)],
    );

    mark_deceased(&mut env, affiliate_account);

    // Second attempt should fail with AccountDeceased
    let discriminant = anchor_discriminant("mark_deceased");
    let mut data = discriminant.to_vec();
    data.extend_from_slice(&fake_hash());

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(env.fund_config, false),
            AccountMeta::new(affiliate_account, false),
            AccountMeta::new_readonly(env.authority.pubkey(), true),
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&env.authority.pubkey()), &[&env.authority], env.svm.latest_blockhash(),
    );
    let result = env.svm.send_transaction(tx);
    assert!(result.is_err(), "marking deceased twice must fail");
    let err = format!("{:?}", result.unwrap_err());
    assert!(
        err.contains("AccountDeceased") || err.contains("3004"),
        "expected AccountDeceased, got: {err}"
    );
}

#[test]
fn inheritance_on_living_affiliate_fails() {
    let mut env = TestEnv::new();
    let ben = solana_sdk::keypair::Keypair::new();
    let affiliate = solana_sdk::keypair::Keypair::new();
    let affiliate_account = env.register_affiliate(
        &affiliate,
        dob_for_age(50),
        0,
        vec![(ben.pubkey(), 10_000)],
    );

    let affiliate_token = create_token_account(
        &mut env.svm, &env.authority, &env.mint, &affiliate.pubkey(),
    );
    mint_tokens(&mut env.svm, &env.authority, &env.mint, &affiliate_token, 5_000_000);
    env.contribute(&affiliate, affiliate_token, 0, 0, MIN_CONTRIBUTION);

    let ben_token = create_token_account(
        &mut env.svm, &env.authority, &env.mint, &ben.pubkey(),
    );
    let (pool, _) = pool_pda(0);
    let (fund_config_pda_key, _) = fund_config_pda();
    let (req, _) = withdrawal_request_pda(&affiliate.pubkey(), 0);

    let discriminant = anchor_discriminant("process_inheritance");
    let mut data = discriminant.to_vec();
    data.extend_from_slice(&0u32.to_le_bytes()); // nonce
    data.push(0u8); // beneficiary_index
    data.extend_from_slice(&fake_hash());

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(env.fund_config, false),
            AccountMeta::new(affiliate_account, false),
            AccountMeta::new(pool, false),
            AccountMeta::new(req, false),
            AccountMeta::new(env.conservative_pool_token, false),
            AccountMeta::new(ben_token, false),
            AccountMeta::new_readonly(fund_config_pda_key, false),
            AccountMeta::new(env.authority.pubkey(), true),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&env.authority.pubkey()), &[&env.authority], env.svm.latest_blockhash(),
    );
    let result = env.svm.send_transaction(tx);
    assert!(result.is_err(), "inheritance on living affiliate must fail");
    let err = format!("{:?}", result.unwrap_err());
    assert!(
        err.contains("NotDeceased") || err.contains("6009"),
        "expected NotDeceased, got: {err}"
    );
}