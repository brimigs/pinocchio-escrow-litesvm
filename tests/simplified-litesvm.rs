#![allow(clippy::too_many_arguments)]

use std::{fs, path::PathBuf};

use litesvm_utils::{
    AssertionHelpers, LiteSVM, ProgramTestExt, Signer, TestHelpers, TransactionHelpers,
};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_sdk::{pubkey, pubkey::Pubkey};
use spl_associated_token_account::get_associated_token_address;
use spl_token_interface::ID as TOKEN_PROGRAM_ID;

const PROGRAM_ID: Pubkey = pubkey!("7h1hJkP8i1H2Q7xPkD8STGvN6dEyxwCxPj3YfJ8p6r7T");
const SYSTEM_PROGRAM_ID: Pubkey = pubkey!("11111111111111111111111111111111");
const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

#[test]
#[ignore = "requires `cargo build-sbf --features bpf-entrypoint` first"]
fn simplified_make_and_take_flow() {
    let mut svm = setup_svm();
    let maker = svm.create_funded_account(10_000_000_000).unwrap();
    let taker = svm.create_funded_account(10_000_000_000).unwrap();

    let mint_a = svm.create_token_mint(&maker, 0).unwrap();
    let mint_b = svm.create_token_mint(&maker, 0).unwrap();

    let maker_a = svm
        .create_associated_token_account(&mint_a.pubkey(), &maker)
        .unwrap();
    let maker_b = svm
        .create_associated_token_account(&mint_b.pubkey(), &maker)
        .unwrap();
    let taker_a = svm
        .create_associated_token_account(&mint_a.pubkey(), &taker)
        .unwrap();
    let taker_b = svm
        .create_associated_token_account(&mint_b.pubkey(), &taker)
        .unwrap();

    svm.mint_to(&mint_a.pubkey(), &maker_a, &maker, 100)
        .unwrap();
    svm.mint_to(&mint_b.pubkey(), &taker_b, &maker, 100)
        .unwrap();

    let seed = 7u64;
    let offered_amount = 40u64;
    let expected_amount = 25u64;
    let escrow = svm.get_pda(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &PROGRAM_ID,
    );
    let vault = get_associated_token_address(&escrow, &mint_a.pubkey());

    let make_result = svm
        .send_instruction(
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
            &[&maker],
        )
        .unwrap();
    make_result.assert_success();

    svm.assert_account_exists(&escrow);
    svm.assert_account_exists(&vault);
    svm.assert_token_balance(&maker_a, 60);
    svm.assert_token_balance(&vault, 40);
    svm.assert_token_balance(&maker_b, 0);
    svm.assert_token_balance(&taker_a, 0);
    svm.assert_token_balance(&taker_b, 100);

    let take_result = svm
        .send_instruction(
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
            &[&taker],
        )
        .unwrap();
    take_result.assert_success();

    svm.assert_token_balance(&maker_a, 60);
    svm.assert_token_balance(&maker_b, 25);
    svm.assert_token_balance(&taker_a, 40);
    svm.assert_token_balance(&taker_b, 75);
    svm.assert_account_closed(&escrow);
    svm.assert_account_closed(&vault);
}

#[test]
#[ignore = "requires `cargo build-sbf --features bpf-entrypoint` first"]
fn simplified_make_and_cancel_flow() {
    let mut svm = setup_svm();
    let maker = svm.create_funded_account(10_000_000_000).unwrap();

    let mint_a = svm.create_token_mint(&maker, 0).unwrap();
    let mint_b = svm.create_token_mint(&maker, 0).unwrap();

    let maker_a = svm
        .create_associated_token_account(&mint_a.pubkey(), &maker)
        .unwrap();
    let maker_b = svm
        .create_associated_token_account(&mint_b.pubkey(), &maker)
        .unwrap();

    svm.mint_to(&mint_a.pubkey(), &maker_a, &maker, 100)
        .unwrap();

    let seed = 11u64;
    let offered_amount = 10u64;
    let expected_amount = 5u64;
    let escrow = svm.get_pda(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &PROGRAM_ID,
    );
    let vault = get_associated_token_address(&escrow, &mint_a.pubkey());

    let make_result = svm
        .send_instruction(
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
            &[&maker],
        )
        .unwrap();
    make_result.assert_success();

    let cancel_result = svm
        .send_instruction(
            cancel_instruction(&maker.pubkey(), &maker_a, &escrow, &vault, &mint_a.pubkey()),
            &[&maker],
        )
        .unwrap();
    cancel_result.assert_success();

    svm.assert_token_balance(&maker_a, 100);
    svm.assert_token_balance(&maker_b, 0);
    svm.assert_account_closed(&escrow);
    svm.assert_account_closed(&vault);
}

fn setup_svm() -> LiteSVM {
    let program_bytes = fs::read(program_path()).expect("build the SBF artifact first");
    let mut svm = LiteSVM::new().with_sysvars().with_default_programs();
    svm.deploy_program(PROGRAM_ID, &program_bytes);
    svm
}

fn program_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("deploy")
        .join("pinocchio_escrow_litesvm.so")
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
