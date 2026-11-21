#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "== fmt"
cargo fmt --all --check

echo "== allow"
if grep -rnE 'allow\((clippy::)?(too_many_lines|cognitive_complexity)' crates/; then
    echo "Запрещено отключать too_many_lines и cognitive_complexity" >&2
    exit 1
fi

echo "== clippy"
cargo clippy -q --workspace --all-targets -- -D warnings
cargo clippy -q -p web --target wasm32-unknown-unknown -- -D warnings

echo "== test"
cargo test -q --workspace

echo "OK"
