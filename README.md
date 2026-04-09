# pinocchio-escrow-litesvm

A minimal SPL-token escrow program built with Pinocchio plus LiteSVM tests.

## Flow

The program supports three instructions:

- `make`: create an escrow PDA, create the PDA-owned vault ATA, and deposit the maker's offered tokens
- `take`: swap the taker's requested tokens for the maker's deposited tokens, then close the vault and escrow
- `cancel`: let the maker reclaim their deposited tokens, then close the vault and escrow

## Build

```bash
cargo build-sbf --features bpf-entrypoint
```

## Test

Host-side unit tests:

```bash
cargo test
```

LiteSVM integration tests after building the program ELF:

```bash
cargo test --test escrow_litesvm -- --ignored
```
