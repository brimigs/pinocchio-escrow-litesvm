#!/usr/bin/env bash
set -euo pipefail

cargo test --test escrow_litesvm --test simplified-litesvm -- --ignored "$@"
