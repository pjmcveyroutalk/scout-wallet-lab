#!/usr/bin/env bash
set -euo pipefail

readonly TOOLCHAIN="1.80.0"

export CARGO_HOME="${CARGO_HOME:-${HOME}/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-${HOME}/.rustup}"
export PATH="${CARGO_HOME}/bin:${PATH}"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required to bootstrap Rust ${TOOLCHAIN} on Vercel." >&2
  exit 1
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "Bootstrapping rustup for the Scout Wallet Lab verification gate..."
  curl --proto "=https" --tlsv1.2 --silent --show-error --fail https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain "${TOOLCHAIN}"
fi

export PATH="${CARGO_HOME}/bin:${PATH}"

echo "Running authoritative Scout Wallet Lab Rust ${TOOLCHAIN} gate..."
bash scripts/preflight-rust.sh

echo "Scout Wallet Lab Rust ${TOOLCHAIN} gate passed."
