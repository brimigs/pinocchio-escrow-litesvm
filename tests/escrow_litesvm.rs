#![allow(clippy::too_many_arguments)]

use std::{fs, path::PathBuf};

use litesvm::LiteSVM;
use solana_program::{program_option::COption, program_pack::Pack};
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token_interface::{
    state::{Account as TokenAccount, AccountState, Mint},
    ID as TOKEN_PROGRAM_ID,
};

const PROGRAM_ID: Pubkey = pubkey!("7h1hJkP8i1H2Q7xPkD8STGvN6dEyxwCxPj3YfJ8p6r7T");
const SYSTEM_PROGRAM_ID: Pubkey = pubkey!("11111111111111111111111111111111");
const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

#[test]
#[ignore = "requires `cargo build-sbf --features bpf-entrypoint` first"]
fn make_and_take_flow() {
    let mut svm = setup_svm();
    let maker = funded_keypair(&mut svm);
    let taker = funded_keypair(&mut svm);

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    create_mint(&mut svm, &mint_a, &maker.pubkey(), 0);
    create_mint(&mut svm, &mint_b, &maker.pubkey(), 0);

    let maker_a = create_token_account(&mut svm, &maker.pubkey(), &mint_a.pubkey(), 100);
    let maker_b = create_token_account(&mut svm, &maker.pubkey(), &mint_b.pubkey(), 0);
    let taker_a = create_token_account(&mut svm, &taker.pubkey(), &mint_a.pubkey(), 0);
    let taker_b = create_token_account(&mut svm, &taker.pubkey(), &mint_b.pubkey(), 100);

    let seed = 7u64;
    let offered_amount = 40u64;
    let expected_amount = 25u64;
    let (escrow, _) = escrow_pda(&maker.pubkey(), seed);
    let vault = get_associated_token_address(&escrow, &mint_a.pubkey());

    send_ix(
        &mut svm,
        &maker,
        &[&maker],
        make_instruction(
            &maker.pubkey(),
            &maker_a,
            &escrow,
            &vault,
            &mint_a.pubkey(),
            &mint_b.pubkey(),
            seed,
            offered_amount,
            expected_amount,
        ),
    );

    assert_eq!(token_balance(&svm, &maker_a), 60);
    assert_eq!(token_balance(&svm, &vault), 40);

    send_ix(
        &mut svm,
        &taker,
        &[&taker],
        take_instruction(
            &taker.pubkey(),
            &taker_b,
            &taker_a,
            &maker.pubkey(),
            &maker_b,
            &escrow,
            &vault,
            &mint_a.pubkey(),
            &mint_b.pubkey(),
        ),
    );

    assert_eq!(token_balance(&svm, &maker_a), 60);
    assert_eq!(token_balance(&svm, &maker_b), 25);
    assert_eq!(token_balance(&svm, &taker_a), 40);
    assert_eq!(token_balance(&svm, &taker_b), 75);
    assert!(svm.get_account(&escrow).is_none());
    assert!(svm.get_account(&vault).is_none());
}

#[test]
#[ignore = "requires `cargo build-sbf --features bpf-entrypoint` first"]
fn make_and_cancel_flow() {
    let mut svm = setup_svm();
    let maker = funded_keypair(&mut svm);

    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    create_mint(&mut svm, &mint_a, &maker.pubkey(), 0);
    create_mint(&mut svm, &mint_b, &maker.pubkey(), 0);

    let maker_a = create_token_account(&mut svm, &maker.pubkey(), &mint_a.pubkey(), 100);
    let maker_b = create_token_account(&mut svm, &maker.pubkey(), &mint_b.pubkey(), 0);

    let seed = 11u64;
    let offered_amount = 10u64;
    let expected_amount = 5u64;
    let (escrow, _) = escrow_pda(&maker.pubkey(), seed);
    let vault = get_associated_token_address(&escrow, &mint_a.pubkey());

    send_ix(
        &mut svm,
        &maker,
        &[&maker],
        make_instruction(
            &maker.pubkey(),
            &maker_a,
            &escrow,
            &vault,
            &mint_a.pubkey(),
            &mint_b.pubkey(),
            seed,
            offered_amount,
            expected_amount,
        ),
    );

    send_ix(
        &mut svm,
        &maker,
        &[&maker],
        cancel_instruction(&maker.pubkey(), &maker_a, &escrow, &vault, &mint_a.pubkey()),
    );

    assert_eq!(token_balance(&svm, &maker_a), 100);
    assert_eq!(token_balance(&svm, &maker_b), 0);
    assert!(svm.get_account(&escrow).is_none());
    assert!(svm.get_account(&vault).is_none());
}

fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new().with_sysvars().with_default_programs();
    let program_bytes = fs::read(program_path()).expect("build the SBF artifact first");
    svm.add_program(PROGRAM_ID, &program_bytes)
        .expect("program should load");
    svm
}

fn program_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("deploy")
        .join("pinocchio_escrow_litesvm.so")
}

fn funded_keypair(svm: &mut LiteSVM) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), 10_000_000_000).unwrap();
    keypair
}

fn send_ix(svm: &mut LiteSVM, payer: &Keypair, signers: &[&Keypair], instruction: Instruction) {
    let mut all_signers: Vec<&dyn Signer> = vec![payer];
    all_signers.extend(signers.iter().copied().map(|signer| signer as &dyn Signer));

    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &all_signers,
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx)
        .expect("transaction should succeed");
}

fn create_mint(svm: &mut LiteSVM, mint: &Keypair, mint_authority: &Pubkey, decimals: u8) {
    let mint_state = Mint {
        mint_authority: COption::Some(*mint_authority),
        supply: 0,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    };

    let mut data = vec![0u8; Mint::LEN];
    mint_state.pack_into_slice(&mut data);

    svm.set_account(
        mint.pubkey(),
        Account {
            lamports: svm.minimum_balance_for_rent_exemption(Mint::LEN),
            data,
            owner: TOKEN_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

fn create_token_account(svm: &mut LiteSVM, owner: &Pubkey, mint: &Pubkey, amount: u64) -> Pubkey {
    let token_address = get_associated_token_address(owner, mint);
    let token_state = TokenAccount {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };

    let mut data = vec![0u8; TokenAccount::LEN];
    token_state.pack_into_slice(&mut data);

    svm.set_account(
        token_address,
        Account {
            lamports: svm.minimum_balance_for_rent_exemption(TokenAccount::LEN),
            data,
            owner: TOKEN_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    token_address
}

fn token_balance(svm: &LiteSVM, token_account: &Pubkey) -> u64 {
    let account = svm
        .get_account(token_account)
        .expect("token account should exist");
    u64::from_le_bytes(account.data[64..72].try_into().unwrap())
}

fn escrow_pda(maker: &Pubkey, seed: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"escrow", maker.as_ref(), &seed.to_le_bytes()],
        &PROGRAM_ID,
    )
}

fn make_instruction(
    maker: &Pubkey,
    maker_deposit: &Pubkey,
    escrow: &Pubkey,
    vault: &Pubkey,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
    seed: u64,
    offered_amount: u64,
    expected_amount: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(25);
    data.push(0);
    data.extend_from_slice(&seed.to_le_bytes());
    data.extend_from_slice(&offered_amount.to_le_bytes());
    data.extend_from_slice(&expected_amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*maker, true),
            AccountMeta::new(*maker_deposit, false),
            AccountMeta::new(*escrow, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new_readonly(*mint_a, false),
            AccountMeta::new_readonly(*mint_b, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
        ],
        data,
    }
}

fn take_instruction(
    taker: &Pubkey,
    taker_send: &Pubkey,
    taker_receive: &Pubkey,
    maker: &Pubkey,
    maker_receive: &Pubkey,
    escrow: &Pubkey,
    vault: &Pubkey,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*taker, true),
            AccountMeta::new(*taker_send, false),
            AccountMeta::new(*taker_receive, false),
            AccountMeta::new(*maker, false),
            AccountMeta::new(*maker_receive, false),
            AccountMeta::new(*escrow, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new_readonly(*mint_a, false),
            AccountMeta::new_readonly(*mint_b, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data: vec![1],
    }
}

fn cancel_instruction(
    maker: &Pubkey,
    maker_receive: &Pubkey,
    escrow: &Pubkey,
    vault: &Pubkey,
    mint_a: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*maker, true),
            AccountMeta::new(*maker_receive, false),
            AccountMeta::new(*escrow, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new_readonly(*mint_a, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data: vec![2],
    }
}
