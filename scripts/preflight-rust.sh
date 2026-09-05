#!/usr/bin/env bash
set -euo pipefail

readonly TOOLCHAIN="1.80.0"

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required to run Scout Wallet Lab preflight." >&2
  exit 1
fi

rustup toolchain install "${TOOLCHAIN}" \
  --profile minimal \
  --component rustfmt \
  --component clippy

cargo_180() {
  rustup run "${TOOLCHAIN}" cargo "$@"
}

echo "Checking exact Rust ${TOOLCHAIN} formatting..."
cargo_180 fmt --all -- --check

echo "Running Clippy..."
cargo_180 clippy --workspace --all-targets -- -D warnings

echo "Running workspace tests..."
cargo_180 test --workspace --all-targets

echo "Scout Wallet Lab Rust ${TOOLCHAIN} preflight passed."
