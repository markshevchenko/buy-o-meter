#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "== fmt"
cargo fmt --all --check

echo "== clippy"
cargo clippy -q --workspace --all-targets -- -D warnings

echo "== test"
cargo test -q --workspace

echo "OK"
