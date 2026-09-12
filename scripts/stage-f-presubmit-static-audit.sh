#!/usr/bin/env bash
set -euo pipefail

readonly CORE_PATH="crates/wallet-engine/src/stage_f_presubmit.rs"
readonly SUBMISSION_METHOD="send""Transaction"
readonly MAINNET_RPC="https://api.""mainnet-beta.solana.com"

fail() {
  echo "STAGE F PRESUBMIT AUDIT FAILED: $1" >&2
  exit 1
}

[[ -f "${CORE_PATH}" ]] || fail "presubmit core is missing"

grep -F 'scout-stage-f-devnet-submission-proof-v1' "${CORE_PATH}" >/dev/null || \
  fail "fixed Stage F proof payload is missing"

grep -F 'STAGE_F_MAX_FEE_LAMPORTS: u64 = 10_000' "${CORE_PATH}" >/dev/null || \
  fail "fixed 10,000-lamport fee ceiling is missing"

grep -F 'STAGE_F_MIN_REMAINING_BALANCE_LAMPORTS: u64 = 1_000_000' "${CORE_PATH}" >/dev/null || \
  fail "remaining-balance floor is missing"

grep -F 'method: "getFeeForMessage"' "${CORE_PATH}" >/dev/null || \
  fail "fee preflight RPC is missing"

grep -F 'method: "simulateTransaction"' "${CORE_PATH}" >/dev/null || \
  fail "signature-verifying simulation RPC is missing"

grep -F 'sig_verify: true' "${CORE_PATH}" >/dev/null || \
  fail "simulation signature verification is missing"

grep -F 'replace_recent_blockhash: false' "${CORE_PATH}" >/dev/null || \
  fail "simulation must not replace the signed recent blockhash"

grep -F 'Zeroizing<Vec<u8>>' "${CORE_PATH}" >/dev/null || \
  fail "signed candidate must remain zeroizing in-memory data"

grep -F 'CandidateAlreadyPrepared' "${CORE_PATH}" >/dev/null || \
  fail "single-candidate fail-closed gate is missing"

grep -F 'CandidateTokenMismatch' "${CORE_PATH}" >/dev/null || \
  fail "candidate token binding is missing"

if grep -F "${SUBMISSION_METHOD}" "${CORE_PATH}" >/dev/null; then
  fail "ledger submission must remain absent from Stage F-A"
fi

if grep -F "${MAINNET_RPC}" "${CORE_PATH}" >/dev/null; then
  fail "Mainnet RPC must remain absent"
fi

if grep -E 'signTransaction|signMessage|signBytes|Clipboard|Intent|Vercel' "${CORE_PATH}" >/dev/null; then
  fail "generic signing or external handoff surface escaped into Stage F-A"
fi

rustfmt +1.80.0 --edition 2021 --check "${CORE_PATH}"

echo "Stage F presubmit static audit passed"
