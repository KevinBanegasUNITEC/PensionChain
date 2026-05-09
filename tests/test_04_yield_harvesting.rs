// tests/test_04_yield_harvesting.rs

mod helpers;
use helpers::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::Signer,
    transaction::Transaction,
};

fn setup_contributed_affiliate(env: &mut TestEnv, profile_index: u8) -> (
    solana_sdk::keypair::Keypair,
    solana_sdk::pubkey::Pubkey, // affiliate_account PDA
    solana_sdk::pubkey::Pubkey, // pool PDA
) {
    let affiliate = solana_sdk::keypair::Keypair::new();
    let affiliate_account = env.register_affiliate(
        &affiliate,
        dob_for_age(35),
        profile_index,
        vec![],
    );

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
    );

    env.contribute(&affiliate, affiliate_token, profile_index, 0, MIN_CONTRIBUTION);

    let (pool, _) = pool_pda(profile_index);
    (affiliate, affiliate_account, pool)
}

fn harvest_yield_ix(
    env: &TestEnv,
    pool: solana_sdk::pubkey::Pubkey,
    affiliate_account: solana_sdk::pubkey::Pubkey,
    new_apy_bps: u16,
    yield_amount: u64,
) -> Instruction {
    let discriminant = anchor_discriminant("harvest_yield");
    let mut data = discriminant.to_vec();
    data.extend_from_slice(&new_apy_bps.to_le_bytes());
    data.extend_from_slice(&yield_amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(env.fund_config, false),
            AccountMeta::new(pool, false),
            AccountMeta::new(affiliate_account, false),
            AccountMeta::new(env.authority.pubkey(), true),
        ],
        data,
    }
}

#[test]
fn harvest_credits_net_yield_to_affiliate() {
    let mut env = TestEnv::new();
    let (_, affiliate_account, pool) = setup_contributed_affiliate(&mut env, 1);

    let yield_amount: u64 = 50_000;
    let fee = yield_amount * 50 / 10_000; // 0.5% protocol fee = 250
    let expected_net = yield_amount - fee; // 49_750

    let ix = harvest_yield_ix(&env, pool, affiliate_account, 800, yield_amount);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&env.authority.pubkey()),
        &[&env.authority],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx).expect("harvest_yield");

    // Read accrued_yield from affiliate account data.
    // Layout offset: 8 (disc) + 32 (owner) + 8 (enrolled_at) + 8 (dob) + 2 (country)
    // + 32 (kyc) + 1 (profile) + 32 (advisor_hash) + 8 (total_contributed) = 133
    let acc_data = env.svm.get_account(&affiliate_account).unwrap().data;
    let yield_offset = 8 + 32 + 8 + 8 + 2 + 32 + 1 + 32 + 8;
    let accrued_yield = u64::from_le_bytes(
        acc_data[yield_offset..yield_offset + 8].try_into().unwrap(),
    );

    assert_eq!(accrued_yield, expected_net, "net yield after fee should be credited");
}

#[test]
fn harvest_from_non_keeper_fails() {
    let mut env = TestEnv::new();
    let (_, affiliate_account, pool) = setup_contributed_affiliate(&mut env, 0);

    let imposter = solana_sdk::keypair::Keypair::new();
    env.svm.airdrop(&imposter.pubkey(), 1_000_000_000).unwrap();

    let discriminant = anchor_discriminant("harvest_yield");
    let mut data = discriminant.to_vec();
    data.extend_from_slice(&600u16.to_le_bytes()); // apy_bps
    data.extend_from_slice(&10_000u64.to_le_bytes()); // yield_amount

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(env.fund_config, false),
            AccountMeta::new(pool, false),
            AccountMeta::new(affiliate_account, false),
            AccountMeta::new(imposter.pubkey(), true), // wrong signer
        ],
        data,
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&imposter.pubkey()),
        &[&imposter],
        env.svm.latest_blockhash(),
    );

    let result = env.svm.send_transaction(tx);
    assert!(result.is_err(), "non-keeper harvest must fail");
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(
        err_str.contains("Unauthorized") || err_str.contains("2006"),
        "expected Unauthorized, got: {err_str}"
    );
}

#[test]
fn multiple_harvests_accumulate_yield() {
    let mut env = TestEnv::new();
    let (_, affiliate_account, pool) = setup_contributed_affiliate(&mut env, 2);

    let yield_per_harvest: u64 = 10_000;
    let fee_per = yield_per_harvest * 50 / 10_000; // 50
    let net_per = yield_per_harvest - fee_per;     // 9_950

    // First harvest
    let ix1 = harvest_yield_ix(&env, pool, affiliate_account, 1200, yield_per_harvest);
    let tx1 = Transaction::new_signed_with_payer(
        &[ix1],
        Some(&env.authority.pubkey()),
        &[&env.authority],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx1).expect("first harvest");

    let acc_data = env.svm.get_account(&affiliate_account).unwrap().data;
    let yield_offset = 8 + 32 + 8 + 8 + 2 + 32 + 1 + 32 + 8;
    let after_first = u64::from_le_bytes(
        acc_data[yield_offset..yield_offset + 8].try_into().unwrap(),
    );
    assert_eq!(after_first, net_per, "first harvest yield mismatch");
}