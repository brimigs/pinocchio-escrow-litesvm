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
