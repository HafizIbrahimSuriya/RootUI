#!/usr/bin/env sh
cd "$(dirname "$0")" || exit 1
command -v cargo >/dev/null 2>&1 || { echo "Install Rust first: https://rustup.rs"; exit 1; }
exec cargo run --release -- "$@"
