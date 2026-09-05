#!/usr/bin/env bash
set -euo pipefail

readonly REPO_ROOT="$(git rev-parse --show-toplevel)"
readonly EXPORTER_PATH="crates/wallet-engine/src/bin/export_observability.rs"

cd "${REPO_ROOT}"

fail() {
  echo "SECURITY TRIPWIRE FAILED: $1" >&2
  exit 1
}

assert_not_tracked() {
  local path="$1"

  if git ls-files --error-unmatch "${path}" >/dev/null 2>&1; then
    fail "generated or secret-bearing artifact is tracked: ${path}"
  fi
}

assert_absent_in_source() {
  local pattern="$1"
  local description="$2"

  if git grep \
    --line-number \
    --fixed-strings \
    -- "${pattern}" \
    ':!scripts/security-tripwires.sh' \
    ':!README.md' \
    >/dev/null 2>&1; then
    fail "${description}"
  fi
}

assert_passphrase_boundary() {
  local matches

  matches="$(
    git grep \
      --line-number \
      --fixed-strings \
      -- "SCOUT_WALLET_PASSPHRASE" \
      ':!scripts/security-tripwires.sh' \
      ':!README.md' \
      2>/dev/null || true
  )"

  if [[ -z "${matches}" ]]; then
    fail "local exporter passphrase boundary is missing"
  fi

  while IFS= read -r match; do
    if [[ "${match}" != "${EXPORTER_PATH}:"* ]]; then
      fail "wallet passphrase environment variable escaped the local exporter boundary"
    fi
  done <<< "${matches}"
}

echo "Checking generated observability artifacts..."

assert_not_tracked "dashboard/wallet-observability.json"
assert_not_tracked "dashboard/.wallet-observability.json.tmp"

echo "Checking network boundary..."

assert_absent_in_source \
  "https://api.mainnet-beta.solana.com" \
  "mainnet RPC endpoint must remain absent"

assert_absent_in_source \
  "sendTransaction" \
  "transaction submission must remain disabled until the Devnet submission gate is explicitly opened"

echo "Checking signer boundary..."

assert_absent_in_source \
  "solana-keypair" \
  "generic Solana keypair dependency is forbidden"

assert_absent_in_source \
  "solana_keypair" \
  "generic Solana keypair API is forbidden"

assert_absent_in_source \
  "get_keypair" \
  "raw keypair access API is forbidden"

assert_absent_in_source \
  "sign_arbitrary" \
  "arbitrary signing API is forbidden"

echo "Checking Vercel trust boundary..."

assert_passphrase_boundary

echo "Checking Devnet lock..."

if git grep \
  --line-number \
  --fixed-strings \
  "https://api.devnet.solana.com" \
  -- crates/wallet-engine/src/lib.rs \
  >/dev/null 2>&1; then
  :
else
  fail "pinned Devnet RPC endpoint is missing"
fi

echo "Scout Wallet Lab security tripwires passed."
