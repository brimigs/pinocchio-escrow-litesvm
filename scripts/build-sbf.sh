#!/usr/bin/env bash
set -euo pipefail

AGAVE_SBF_BIN="${AGAVE_SBF_BIN:-/tmp/agave-3.1.5/active_release/bin}"
BUILD_CMD="${AGAVE_SBF_BIN}/cargo-build-sbf"

if [[ ! -x "${BUILD_CMD}" ]]; then
  echo "missing SBF builder: ${BUILD_CMD}" >&2
  echo "set AGAVE_SBF_BIN to an Agave bin directory that contains cargo-build-sbf" >&2
  exit 1
fi

exec "${BUILD_CMD}" --features bpf-entrypoint "$@"
