use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_associated_token_account::instructions::CreateIdempotent;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{
    instructions::{CloseAccount, Transfer},
    state::{Account as TokenAccount, Mint},
};

use crate::{
    error::EscrowError,
    instruction::{EscrowInstruction, MakeArgs},
    state::{EscrowState, ESCROW_SEED_PREFIX},
};

#[inline(never)]
pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    match EscrowInstruction::unpack(instruction_data)? {
        EscrowInstruction::Make(args) => process_make(program_id, accounts, args),
        EscrowInstruction::Take => process_take(program_id, accounts),
        EscrowInstruction::Cancel => process_cancel(program_id, accounts),
    }
}

fn process_make(
    program_id: &Address,
    accounts: &mut [AccountView],
    args: MakeArgs,
) -> ProgramResult {
    let [maker, maker_deposit, escrow, vault, mint_a, mint_b, token_program, system_program, ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    assert_signer(maker)?;
    assert_writable(maker)?;
    assert_writable(maker_deposit)?;
    assert_writable(escrow)?;
    assert_writable(vault)?;
    assert_program(token_program, &pinocchio_token::ID)?;
    assert_program(system_program, &pinocchio_system::ID)?;

    let mint_a_address = mint_a.address().clone();
    let mint_b_address = mint_b.address().clone();

    Mint::from_account_view(mint_a)?;
    Mint::from_account_view(mint_b)?;
    assert_token_account(
        maker_deposit,
        maker.address(),
        &mint_a_address,
        args.offered_amount,
    )?;

    let seed_bytes = args.seed.to_le_bytes();
    let (expected_escrow, bump) = Address::find_program_address(
        &[ESCROW_SEED_PREFIX, maker.address().as_ref(), &seed_bytes],
        program_id,
    );

    if escrow.address() != &expected_escrow {
        return Err(ProgramError::InvalidSeeds);
    }

    if escrow.lamports() != 0 {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent = Rent::get()?;
    // Pinocchio's current Rent helper returns the base rent figure; the runtime
    // rent-exempt threshold is effectively 2x that amount.
    let lamports = rent
        .try_minimum_balance(EscrowState::LEN)?
        .checked_mul(2)
        .ok_or(EscrowError::ArithmeticOverflow)?
        .max(1);
    with_escrow_signer(maker.address(), args.seed, bump, |signers| {
        CreateAccount {
            from: maker,
            to: escrow,
            lamports,
            space: EscrowState::LEN as u64,
            owner: program_id,
        }
        .invoke_signed(signers)
    })?;

    let state = EscrowState::new(
        bump,
        args.seed,
        maker.address().clone(),
        mint_a_address.clone(),
        mint_b_address,
        args.offered_amount,
        args.expected_amount,
    );

    {
        let mut escrow_data = escrow.try_borrow_mut()?;
        state.pack(&mut escrow_data)?;
    }

    CreateIdempotent {
        funding_account: maker,
        account: vault,
        wallet: escrow,
        mint: mint_a,
        system_program,
        token_program,
    }
    .invoke()?;

    let expected_vault = associated_token_address(escrow.address(), &mint_a_address);
    assert_vault(vault, escrow.address(), &mint_a_address, &expected_vault)?;

    Transfer::new(maker_deposit, vault, maker, args.offered_amount).invoke()?;

    Ok(())
}

fn process_take(program_id: &Address, accounts: &mut [AccountView]) -> ProgramResult {
    let [taker, taker_send, taker_receive, maker, maker_receive, escrow, vault, mint_a, mint_b, token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    assert_signer(taker)?;
    assert_writable(taker_send)?;
    assert_writable(taker_receive)?;
    assert_writable(maker)?;
    assert_writable(maker_receive)?;
    assert_writable(escrow)?;
    assert_writable(vault)?;
    assert_program(token_program, &pinocchio_token::ID)?;

    let state = load_state(program_id, escrow)?;

    if maker.address() != &state.maker {
        return Err(EscrowError::InvalidMaker.into());
    }

    if mint_a.address() != &state.mint_a || mint_b.address() != &state.mint_b {
        return Err(EscrowError::InvalidMint.into());
    }

    Mint::from_account_view(mint_a)?;
    Mint::from_account_view(mint_b)?;
    assert_token_account(
        taker_send,
        taker.address(),
        &state.mint_b,
        state.expected_amount,
    )?;
    assert_token_account(taker_receive, taker.address(), &state.mint_a, 0)?;
    assert_token_account(maker_receive, maker.address(), &state.mint_b, 0)?;

    let expected_vault = associated_token_address(escrow.address(), &state.mint_a);
    let vault_amount = assert_vault(vault, escrow.address(), &state.mint_a, &expected_vault)?;
    if vault_amount != state.offered_amount {
        return Err(EscrowError::InvalidVault.into());
    }

    Transfer::new(taker_send, maker_receive, taker, state.expected_amount).invoke()?;

    with_escrow_signer(&state.maker, state.seed, state.bump, |signers| {
        Transfer::new(vault, taker_receive, escrow, state.offered_amount).invoke_signed(signers)
    })?;
    with_escrow_signer(&state.maker, state.seed, state.bump, |signers| {
        CloseAccount::new(vault, maker, escrow).invoke_signed(signers)
    })?;
    close_program_account(escrow, maker)?;

    Ok(())
}

fn process_cancel(program_id: &Address, accounts: &mut [AccountView]) -> ProgramResult {
    let [maker, maker_receive, escrow, vault, mint_a, token_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    assert_signer(maker)?;
    assert_writable(maker)?;
    assert_writable(maker_receive)?;
    assert_writable(escrow)?;
    assert_writable(vault)?;
    assert_program(token_program, &pinocchio_token::ID)?;

    let state = load_state(program_id, escrow)?;

    if maker.address() != &state.maker {
        return Err(EscrowError::InvalidMaker.into());
    }

    if mint_a.address() != &state.mint_a {
        return Err(EscrowError::InvalidMint.into());
    }

    Mint::from_account_view(mint_a)?;
    assert_token_account(maker_receive, maker.address(), &state.mint_a, 0)?;

    let expected_vault = associated_token_address(escrow.address(), &state.mint_a);
    let vault_amount = assert_vault(vault, escrow.address(), &state.mint_a, &expected_vault)?;
    if vault_amount != state.offered_amount {
        return Err(EscrowError::InvalidVault.into());
    }

    with_escrow_signer(&state.maker, state.seed, state.bump, |signers| {
        Transfer::new(vault, maker_receive, escrow, state.offered_amount).invoke_signed(signers)
    })?;
    with_escrow_signer(&state.maker, state.seed, state.bump, |signers| {
        CloseAccount::new(vault, maker, escrow).invoke_signed(signers)
    })?;
    close_program_account(escrow, maker)?;

    Ok(())
}

fn load_state(program_id: &Address, escrow: &AccountView) -> Result<EscrowState, ProgramError> {
    if !escrow.owned_by(program_id) {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let data = escrow.try_borrow()?;
    let state = EscrowState::unpack(&data)?;
    let (expected_address, expected_bump) = state.escrow_address(program_id);

    if escrow.address() != &expected_address || state.bump != expected_bump {
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(state)
}

fn assert_program(account: &AccountView, expected: &Address) -> ProgramResult {
    if account.address() != expected {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

fn assert_signer(account: &AccountView) -> ProgramResult {
    if !account.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

fn assert_writable(account: &AccountView) -> ProgramResult {
    if !account.is_writable() {
        return Err(ProgramError::Immutable);
    }
    Ok(())
}

fn assert_token_account(
    account: &AccountView,
    expected_owner: &Address,
    expected_mint: &Address,
    minimum_amount: u64,
) -> Result<u64, ProgramError> {
    let account_state = TokenAccount::from_account_view(account)?;

    if account_state.owner() != expected_owner || account_state.mint() != expected_mint {
        return Err(EscrowError::InvalidTokenAccount.into());
    }

    if account_state.amount() < minimum_amount {
        return Err(EscrowError::InvalidAmount.into());
    }

    if !account_state.is_initialized() || account_state.is_frozen() {
        return Err(EscrowError::InvalidTokenAccount.into());
    }

    Ok(account_state.amount())
}

fn assert_vault(
    vault: &AccountView,
    escrow: &Address,
    mint: &Address,
    expected_address: &Address,
) -> Result<u64, ProgramError> {
    if vault.address() != expected_address {
        return Err(EscrowError::InvalidVault.into());
    }

    assert_token_account(vault, escrow, mint, 0)
}

fn associated_token_address(wallet: &Address, mint: &Address) -> Address {
    Address::find_program_address(
        &[wallet.as_ref(), pinocchio_token::ID.as_ref(), mint.as_ref()],
        &pinocchio_associated_token_account::ID,
    )
    .0
}

fn close_program_account(
    account: &mut AccountView,
    destination: &mut AccountView,
) -> ProgramResult {
    let destination_lamports = destination
        .lamports()
        .checked_add(account.lamports())
        .ok_or(EscrowError::ArithmeticOverflow)?;

    destination.set_lamports(destination_lamports);
    account.set_lamports(0);
    account.close()?;

    Ok(())
}

fn with_escrow_signer<F>(maker: &Address, seed: u64, bump: u8, f: F) -> ProgramResult
where
    F: FnOnce(&[Signer<'_, '_>]) -> ProgramResult,
{
    let seed_bytes = seed.to_le_bytes();
    let bump_bytes = [bump];
    let seeds = [
        Seed::from(ESCROW_SEED_PREFIX),
        Seed::from(maker.as_ref()),
        Seed::from(seed_bytes.as_slice()),
        Seed::from(bump_bytes.as_slice()),
    ];

    let signers = [Signer::from(&seeds)];
    f(&signers)
}
