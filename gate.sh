#!/bin/sh
# The gate: what must pass before anything lands (docs/agents/afkconvoy.md).
set -e
cd "$(dirname "$0")"
cargo test --quiet
./verify.sh
