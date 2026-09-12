#!/usr/bin/env bash
set -euo pipefail

readonly RESOLUTION_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBReadOnlyResolution.kt"
readonly GATE_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBGateActivity.kt"
readonly STATUS_CLIENT_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBReadOnlyStatusClient.kt"
readonly SUBMISSION_METHOD="send""Transaction"
readonly MAINNET_RPC="https://api.""mainnet-beta.solana.com"

fail() {
  echo "STAGE F-B READINESS AUDIT FAILED: $1" >&2
  exit 1
}

[[ -f "${RESOLUTION_PATH}" ]] || fail "read-only resolution model is missing"
[[ -f "${GATE_PATH}" ]] || fail "Stage F-B gate activity is missing"
[[ -f "${STATUS_CLIENT_PATH}" ]] || fail "Stage F-B read-only status client is missing"

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
grep -F 'NativeBridge.devnetBlockHeight()' "${GATE_PATH}" >/dev/null || \
  fail "Stage F-B gate read-only block-height observation is missing"
grep -F 'StageFBReadOnlyStatusClient.fetch(' "${GATE_PATH}" >/dev/null || \
  fail "Stage F-B gate read-only signature observation is missing"
grep -F 'getSignatureStatuses' "${STATUS_CLIENT_PATH}" >/dev/null || \
  fail "Stage F-B read-only signature-status method is missing"
grep -F 'NativeBridge.rpcEndpoint()' "${STATUS_CLIENT_PATH}" >/dev/null || \
  fail "Stage F-B status client must use the native endpoint accessor"

for path in "${RESOLUTION_PATH}" "${GATE_PATH}" "${STATUS_CLIENT_PATH}"; do
  for pattern in \
    "${SUBMISSION_METHOD}" \
    "${MAINNET_RPC}" \
    "passphrase" \
    "recovery" \
    "wire_transaction" \
    "signedBytes" \
    "ClipboardManager" \
    "signTransaction" \
    "signMessage" \
    "signBytes" \
    "createLockedDevnetVault" \
    "rekeyLockedDevnetVault" \
    "exportLockedVaultRecoveryWords"; do
    if grep -F "${pattern}" "${path}" >/dev/null; then
      fail "forbidden capability escaped into read-only readiness surface: ${pattern}"
    fi
  done
done

if grep -F 'NativeBridge.' "${RESOLUTION_PATH}" >/dev/null; then
  fail "pure read-only resolution model must not call NativeBridge"
fi

if grep -F 'NativeBridge.' "${GATE_PATH}" | grep -v -F 'NativeBridge.devnetBlockHeight()' >/dev/null; then
  fail "Stage F-B gate may call only the native read-only block-height capability"
fi

if grep -F 'NativeBridge.' "${STATUS_CLIENT_PATH}" | grep -v -F 'NativeBridge.rpcEndpoint()' >/dev/null; then
  fail "Stage F-B status client may call only the native read-only endpoint accessor"
fi

echo "Stage F-B read-only readiness audit passed"
