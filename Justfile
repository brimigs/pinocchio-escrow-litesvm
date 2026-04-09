set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

build:
    ./scripts/build-sbf.sh

test:
    cargo test

test-litesvm: build
    ./scripts/test-litesvm.sh

toolchain-info:
    ./scripts/toolchain-info.sh
