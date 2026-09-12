#!/usr/bin/env bash
set -euo pipefail

readonly RESOLUTION_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBReadOnlyResolution.kt"
readonly GATE_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBGateActivity.kt"
readonly SUBMISSION_METHOD="send""Transaction"
readonly MAINNET_RPC="https://api.""mainnet-beta.solana.com"

fail() {
  echo "STAGE F-B READINESS AUDIT FAILED: $1" >&2
  exit 1
}

[[ -f "${RESOLUTION_PATH}" ]] || fail "read-only resolution model is missing"
[[ -f "${GATE_PATH}" ]] || fail "Stage F-B gate activity is missing"

grep -F 'data object Pending' "${RESOLUTION_PATH}" >/dev/null || \
  fail "pending resolution state is missing"
grep -F 'data class Processed' "${RESOLUTION_PATH}" >/dev/null || \
  fail "processed resolution state is missing"
grep -F 'data class Confirmed' "${RESOLUTION_PATH}" >/dev/null || \
  fail "confirmed resolution state is missing"
grep -F 'data class Finalized' "${RESOLUTION_PATH}" >/dev/null || \
  fail "finalized resolution state is missing"
grep -F 'data class Failed' "${RESOLUTION_PATH}" >/dev/null || \
  fail "failed resolution state is missing"
grep -F 'data object Expired' "${RESOLUTION_PATH}" >/dev/null || \
  fail "expired resolution state is missing"
grep -F 'currentBlockHeight > lastValidBlockHeight' "${RESOLUTION_PATH}" >/dev/null || \
  fail "blockhash expiry boundary is missing"
grep -F 'hasExecutionError' "${RESOLUTION_PATH}" >/dev/null || \
  fail "execution-failure classification is missing"

grep -F 'IMPLEMENTATION GATE — NOT ARMED' "${GATE_PATH}" >/dev/null || \
  fail "Stage F-B gate must remain not armed"

for path in "${RESOLUTION_PATH}" "${GATE_PATH}"; do
  for pattern in \
    "${SUBMISSION_METHOD}" \
    "${MAINNET_RPC}" \
    "NativeBridge" \
    "passphrase" \
    "recovery" \
    "wire_transaction" \
    "signedBytes" \
    "ClipboardManager"; do
    if grep -F "${pattern}" "${path}" >/dev/null; then
      fail "forbidden capability escaped into read-only readiness surface: ${pattern}"
    fi
  done
done

echo "Stage F-B read-only readiness audit passed"
