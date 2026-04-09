#!/usr/bin/env bash
set -euo pipefail

AGAVE_SBF_BIN="${AGAVE_SBF_BIN:-/tmp/agave-3.1.5/active_release/bin}"
SOLANA_CMD="${AGAVE_SBF_BIN}/solana"
BUILD_CMD="${AGAVE_SBF_BIN}/cargo-build-sbf"

echo "AGAVE_SBF_BIN=${AGAVE_SBF_BIN}"

if [[ -x "${SOLANA_CMD}" ]]; then
  "${SOLANA_CMD}" --version
else
  echo "missing: ${SOLANA_CMD}" >&2
fi

if [[ -x "${BUILD_CMD}" ]]; then
  "${BUILD_CMD}" --version
else
  echo "missing: ${BUILD_CMD}" >&2
fi
