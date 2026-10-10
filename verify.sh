#!/bin/sh
# Verifies the refinement types with Flux (https://flux-rs.github.io/flux).
# Needs `cargo-flux`, `fixpoint` and `z3` installed.
set -e
cd "$(dirname "$0")"
# cargo-flux re-invokes `cargo +<its nightly>`, which only rustup's cargo
# understands, and rustup needs some toolchain to start from.
PATH="$HOME/.cargo/bin:$PATH"
: "${RUSTUP_TOOLCHAIN:=$(rustup toolchain list | head -1 | cut -d' ' -f1)}"
export PATH RUSTUP_TOOLCHAIN
exec cargo flux "$@"
