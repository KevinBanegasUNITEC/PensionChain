// tests/helpers.rs
// Shared test utilities for PensionChain LiteSVM tests.

use anchor_lang::{prelude::Pubkey, system_program, AnchorSerialize, InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    keypair::Keypair,
    pubkey::pubkey,
    signature::Signer,
    transaction::Transaction,
};
use spl_token::instruction as token_ix;

// ── Program ID ────────────────────────────────────────────────────────────────

pub const PROGRAM_ID: Pubkey = pubkey!("PensNvXkBkr2TaQnNQT7ZQhCWTwVx3f6sBuFhS7Xpump");

// ── Test constants ────────────────────────────────────────────────────────────

pub const MIN_CONTRIBUTION: u64 = 1_000_000; // 1 USDC (6 decimals)
pub const CONTRIBUTION_PERIOD: i64 = 30;     // 30 seconds (fast for tests)
pub const MIN_RETIREMENT_AGE: u8 = 65;
pub const MIN_CONTRIBUTION_YEARS: u8 = 20;
pub const TOKEN_DECIMALS: u8 = 6;

// ── PDA derivation ────────────────────────────────────────────────────────────

pub fn fund_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"fund_config"], &PROGRAM_ID)
}

pub fn affiliate_pda(affiliate: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"affiliate", affiliate.as_ref()], &PROGRAM_ID)
}

pub fn pool_pda(risk_profile_index: u8) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"pool", &[risk_profile_index]], &PROGRAM_ID)
}

pub fn contribution_record_pda(affiliate: &Pubkey, period_index: u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"contribution", affiliate.as_ref(), &period_index.to_le_bytes()],
        &PROGRAM_ID,
    )
}

pub fn withdrawal_request_pda(affiliate: &Pubkey, nonce: u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"withdrawal", affiliate.as_ref(), &nonce.to_le_bytes()],
        &PROGRAM_ID,
    )
}

// ── Token helpers ─────────────────────────────────────────────────────────────

/// Create a mint and return its pubkey. Payer is also the mint authority.
pub fn create_mint(svm: &mut LiteSVM, payer: &Keypair) -> Pubkey {
    let mint = Keypair::new();
    let rent = svm.minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN);

    let create_account_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        spl_token::state::Mint::LEN as u64,
        &spl_token::id(),
    );
    let init_mint_ix = token_ix::initialize_mint(
        &spl_token::id(),
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        TOKEN_DECIMALS,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[create_account_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &[payer, &mint],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("create mint");
    mint.pubkey()
}

/// Create a token account owned by `owner`.
pub fn create_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Pubkey {
    let account = Keypair::new();
    let rent =
        svm.minimum_balance_for_rent_exemption(spl_token::state::Account::LEN);

    let create_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &account.pubkey(),
        rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    );
    let init_ix = token_ix::initialize_account(
        &spl_token::id(),
        &account.pubkey(),
        mint,
        owner,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[create_ix, init_ix],
        Some(&payer.pubkey()),
        &[payer, &account],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("create token account");
    account.pubkey()
}

/// Mint `amount` tokens into `destination`.
pub fn mint_tokens(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    destination: &Pubkey,
    amount: u64,
) {
    let ix = token_ix::mint_to(
        &spl_token::id(),
        mint,
        destination,
        &payer.pubkey(),
        &[],
        amount,
    )
    .unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("mint tokens");
}

/// Read token account balance.
pub fn token_balance(svm: &LiteSVM, account: &Pubkey) -> u64 {
    let data = svm.get_account(account).unwrap().data;
    let state = spl_token::state::Account::unpack(&data).unwrap();
    state.amount
}

// ── Fake data helpers ─────────────────────────────────────────────────────────

pub fn fake_hash() -> [u8; 32] {
    let mut h = [0u8; 32];
    for (i, b) in h.iter_mut().enumerate() {
        *b = (i + 1) as u8;
    }
    h
}

pub fn zero_hash() -> [u8; 32] {
    [0u8; 32]
}

/// Unix timestamp for someone born `age` years ago.
pub fn dob_for_age(age: i64) -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    now - age * 365 * 24 * 3600
}

// ── LiteSVM environment setup ─────────────────────────────────────────────────

pub struct TestEnv {
    pub svm: LiteSVM,
    pub authority: Keypair,
    pub mint: Pubkey,
    pub treasury: Pubkey,
    pub fund_config: Pubkey,
    pub conservative_pool: Pubkey,
    pub balanced_pool: Pubkey,
    pub aggressive_pool: Pubkey,
    pub conservative_pool_token: Pubkey,
    pub balanced_pool_token: Pubkey,
    pub aggressive_pool_token: Pubkey,
}

impl TestEnv {
    /// Boot a full fund environment: mint, treasury, fund_config, all 3 pools.
    pub fn new() -> Self {
        let mut svm = LiteSVM::new();
        // Load the compiled program .so — in CI this is built by `cargo build-sbf`.
        svm.add_program_from_file(
            PROGRAM_ID,
            "../../target/deploy/pension_chain.so",
        )
        .expect("load pension_chain program");

        let authority = Keypair::new();
        svm.airdrop(&authority.pubkey(), 100_000_000_000).unwrap(); // 100 SOL

        let mint = create_mint(&mut svm, &authority);
        let (fund_config, _) = fund_config_pda();

        // Accounts owned by the fund PDA
        let treasury = create_token_account(&mut svm, &authority, &mint, &fund_config);
        let conservative_pool_token =
            create_token_account(&mut svm, &authority, &mint, &fund_config);
        let balanced_pool_token =
            create_token_account(&mut svm, &authority, &mint, &fund_config);
        let aggressive_pool_token =
            create_token_account(&mut svm, &authority, &mint, &fund_config);

        let (conservative_pool, _) = pool_pda(0);
        let (balanced_pool, _) = pool_pda(1);
        let (aggressive_pool, _) = pool_pda(2);

        let mut env = TestEnv {
            svm,
            authority,
            mint,
            treasury,
            fund_config,
            conservative_pool,
            balanced_pool,
            aggressive_pool,
            conservative_pool_token,
            balanced_pool_token,
            aggressive_pool_token,
        };

        env.initialize_fund();
        env.initialize_all_pools();
        env
    }

    fn initialize_fund(&mut self) {
        // Build the initialize_fund instruction manually via anchor IDL discriminant.
        // Discriminant = sha256("global:initialize_fund")[0..8]
        let discriminant = anchor_discriminant("initialize_fund");

        let mut data = discriminant.to_vec();
        data.extend_from_slice(&MIN_CONTRIBUTION.to_le_bytes());
        data.extend_from_slice(&CONTRIBUTION_PERIOD.to_le_bytes());
        data.push(MIN_RETIREMENT_AGE);
        data.push(MIN_CONTRIBUTION_YEARS);

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(self.fund_config, false),
                AccountMeta::new_readonly(self.mint, false),
                AccountMeta::new_readonly(self.treasury, false),
                AccountMeta::new(self.authority.pubkey(), true),
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
            data,
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.authority.pubkey()),
            &[&self.authority],
            self.svm.latest_blockhash(),
        );
        self.svm.send_transaction(tx).expect("initialize_fund");
    }

    fn initialize_pool(&mut self, profile_index: u8, token_account: Pubkey) {
        let (pool, _) = pool_pda(profile_index);
        let discriminant = anchor_discriminant("initialize_pool");

        let mut data = discriminant.to_vec();
        // RiskProfile as u8 — Anchor serializes unit enum variants as their index (u8 via borsh)
        data.push(profile_index);

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new_readonly(self.fund_config, false),
                AccountMeta::new(pool, false),
                AccountMeta::new_readonly(token_account, false),
                AccountMeta::new(self.authority.pubkey(), true),
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
            data,
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.authority.pubkey()),
            &[&self.authority],
            self.svm.latest_blockhash(),
        );
        self.svm
            .send_transaction(tx)
            .expect(&format!("initialize_pool({})", profile_index));
    }

    fn initialize_all_pools(&mut self) {
        let tokens = [
            (0u8, self.conservative_pool_token),
            (1u8, self.balanced_pool_token),
            (2u8, self.aggressive_pool_token),
        ];
        for (idx, token_account) in tokens {
            self.initialize_pool(idx, token_account);
        }
    }

    /// Helper: register an affiliate with the given profile and DOB.
    pub fn register_affiliate(
        &mut self,
        affiliate: &Keypair,
        date_of_birth: i64,
        profile_index: u8,
        beneficiaries: Vec<(Pubkey, u16)>, // (wallet, share_bps)
    ) -> Pubkey {
        self.svm
            .airdrop(&affiliate.pubkey(), 10_000_000_000)
            .unwrap();

        let (affiliate_account, _) = affiliate_pda(&affiliate.pubkey());
        let (pool, _) = pool_pda(profile_index);
        let discriminant = anchor_discriminant("register_affiliate");

        let mut data = discriminant.to_vec();
        data.extend_from_slice(&date_of_birth.to_le_bytes());
        data.extend_from_slice(&[72u8, 78u8]); // "HN"
        data.extend_from_slice(&fake_hash());   // kyc_doc_hash
        data.push(profile_index);               // risk_profile
        data.extend_from_slice(&fake_hash());   // advisor_session_hash
        // Vec<Beneficiary> — length prefix then each entry
        let ben_count = beneficiaries.len() as u32;
        data.extend_from_slice(&ben_count.to_le_bytes());
        for (wallet, share_bps) in &beneficiaries {
            data.extend_from_slice(wallet.as_ref());
            data.extend_from_slice(&share_bps.to_le_bytes());
        }

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new_readonly(self.fund_config, false),
                AccountMeta::new(affiliate_account, false),
                AccountMeta::new(pool, false),
                AccountMeta::new(affiliate.pubkey(), true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data,
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&affiliate.pubkey()),
            &[affiliate],
            self.svm.latest_blockhash(),
        );
        self.svm.send_transaction(tx).expect("register_affiliate");
        affiliate_account
    }

    /// Helper: contribute for period 0.
    pub fn contribute(
        &mut self,
        affiliate: &Keypair,
        affiliate_token: Pubkey,
        profile_index: u8,
        period_index: u32,
        amount: u64,
    ) {
        let (affiliate_account, _) = affiliate_pda(&affiliate.pubkey());
        let (pool, _) = pool_pda(profile_index);
        let pool_token = match profile_index {
            0 => self.conservative_pool_token,
            1 => self.balanced_pool_token,
            _ => self.aggressive_pool_token,
        };
        let (record, _) =
            contribution_record_pda(&affiliate.pubkey(), period_index);
        let discriminant = anchor_discriminant("contribute");

        let mut data = discriminant.to_vec();
        data.extend_from_slice(&period_index.to_le_bytes());
        data.extend_from_slice(&amount.to_le_bytes());

        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new_readonly(self.fund_config, false),
                AccountMeta::new(affiliate_account, false),
                AccountMeta::new(pool, false),
                AccountMeta::new(record, false),
                AccountMeta::new(affiliate_token, false),
                AccountMeta::new(pool_token, false),
                AccountMeta::new(affiliate.pubkey(), true),
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
            data,
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&affiliate.pubkey()),
            &[affiliate],
            self.svm.latest_blockhash(),
        );
        self.svm.send_transaction(tx).expect("contribute");
    }
}

/// Compute the 8-byte Anchor instruction discriminant for a given name.
/// Formula: sha256("global:{name}")[0..8]
pub fn anchor_discriminant(name: &str) -> [u8; 8] {
    use std::convert::TryInto;
    let preimage = format!("global:{name}");
    let hash = solana_sdk::hash::hash(preimage.as_bytes());
    hash.to_bytes()[..8].try_into().unwrap()
}