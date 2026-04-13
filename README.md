# pinocchio-escrow-litesvm

A minimal SPL-token escrow program built with Pinocchio plus LiteSVM tests.

## Flow

The program supports three instructions:

- `make`: create an escrow PDA, create the PDA-owned vault ATA, and deposit the maker's offered tokens
- `take`: swap the taker's requested tokens for the maker's deposited tokens, then close the vault and escrow
- `cancel`: let the maker reclaim their deposited tokens, then close the vault and escrow

## Build

```bash
./scripts/build-sbf.sh
```

## Test

Host-side unit tests:

```bash
cargo test
```

LiteSVM integration tests after building the program ELF:

```bash
./scripts/test-litesvm.sh
```

## LiteSVM vs litesvm-utils

This repo now includes both styles of LiteSVM integration test:

- [tests/escrow_litesvm.rs](/tests/escrow_litesvm.rs) uses raw `litesvm`
- [tests/simplified-litesvm.rs](/tests/simplified-litesvm.rs) uses `litesvm-utils`

The raw `litesvm` test is lower level. It manually:

- loads the program
- creates and funds accounts
- builds mint and token account state
- constructs and signs transactions
- reads account data directly for assertions

The `litesvm-utils` test keeps the same escrow coverage, but replaces most of that boilerplate with helpers like:

- `create_funded_account`
- `create_token_mint`
- `create_associated_token_account`
- `mint_to`
- `get_pda`
- `send_instruction`
- `assert_token_balance`
- `assert_account_closed`

Use the raw `litesvm` test when you want full control over account setup and runtime details. Use the `litesvm-utils` test when you want the same integration coverage with less ceremony and clearer intent.

## Toolchain

This repo is pinned to a newer Agave SBF toolchain than the globally installed
`cargo build-sbf` on this machine.

By default the wrapper scripts expect:

```bash
/tmp/agave-3.1.5/active_release/bin
```

Override that path with `AGAVE_SBF_BIN` if your Agave install lives elsewhere:

```bash
AGAVE_SBF_BIN=/path/to/agave/bin ./scripts/build-sbf.sh
```

If you use `just`, the same workflow is available via:

```bash
just build
just test
just test-litesvm
just toolchain-info
```
