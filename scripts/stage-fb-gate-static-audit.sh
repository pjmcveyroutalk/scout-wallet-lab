#!/usr/bin/env bash
set -euo pipefail

readonly ACTIVITY_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBGateActivity.kt"
readonly STATUS_CLIENT_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBReadOnlyStatusClient.kt"
readonly GUARD_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBAttemptGuard.kt"
readonly HUB_PATH="android/app/src/main/java/com/routalk/scoutoperator/OperatorHubActivity.kt"
readonly MANIFEST_PATH="android/app/src/main/AndroidManifest.xml"
readonly MAINNET_RPC="https://api.""mainnet-beta.solana.com"
readonly SUBMISSION_METHOD="send""Transaction"

fail() {
  echo "STAGE F-B GATE AUDIT FAILED: $1" >&2
  exit 1
}

for path in \
  "${ACTIVITY_PATH}" \
  "${STATUS_CLIENT_PATH}" \
  "${GUARD_PATH}" \
  "${HUB_PATH}" \
  "${MANIFEST_PATH}"; do
  [[ -f "${path}" ]] || fail "required Stage F-B gate file is missing: ${path}"
done

grep -F 'SCOUT STAGE F-B' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate title is missing"

grep -F 'DEVNET ONLY' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B Devnet-only statement is missing"

grep -F 'MAINNET — DISABLED' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B Mainnet-disabled statement is missing"

grep -F 'ARBITRARY SIGNING — DISABLED' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B arbitrary-signing-disabled statement is missing"

grep -F 'IMPLEMENTATION GATE — NOT ARMED' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B not-armed status is missing"

grep -F 'REFRESH READ-ONLY RESOLUTION' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B manual read-only resolution control is missing"

grep -F 'NativeBridge.devnetBlockHeight()' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B read-only block-height observation is missing"

grep -F 'StageFBReadOnlyStatusClient.fetch(' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B read-only signature-status observation is missing"

grep -F 'guard.updateFromResolution(' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B public resolution persistence path is missing"

grep -F 'StageFBAttemptGuard(this).load()' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate must expose the persisted one-attempt guard state"

grep -F 'ONE-ATTEMPT GUARD — CLEAR' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate clear guard state is missing"

grep -F 'ONE-ATTEMPT GUARD — CORRUPT / FAIL CLOSED' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate corrupt guard fail-closed state is missing"

grep -F 'ONE-ATTEMPT GUARD — RECORD PRESENT' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate recorded-attempt state is missing"

grep -F 'Candidate fingerprint SHA-256: ' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate must expose the persisted public candidate fingerprint"

grep -F 'loaded.record.candidateFingerprintSha256' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate must bind resolution to the persisted candidate fingerprint"

grep -F 'Review receipt SHA-256: ' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate must expose the persisted public review receipt"

grep -F 'loaded.record.reviewReceiptSha256' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate must bind resolution to the persisted review receipt"

grep -F 'BINDING OBSERVATION ONLY — EXECUTION NOT AUTHORIZED' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate must state that binding observation does not authorize execution"

grep -F 'getSignatureStatuses' "${STATUS_CLIENT_PATH}" >/dev/null || \
  fail "Stage F-B read-only signature status RPC is missing"

grep -F 'NativeBridge.rpcEndpoint()' "${STATUS_CLIENT_PATH}" >/dev/null || \
  fail "Stage F-B status client must derive its endpoint from the native Devnet boundary"

grep -F 'https://api.devnet.solana.com' "${STATUS_CLIENT_PATH}" >/dev/null || \
  fail "Stage F-B status client must pin the expected Devnet endpoint"

grep -F 'searchTransactionHistory' "${STATUS_CLIENT_PATH}" >/dev/null || \
  fail "Stage F-B status client must support read-only historical resolution"

grep -F 'StageFBGateActivity::class.java' "${HUB_PATH}" >/dev/null || \
  fail "Stage F-B operator hub entry is missing"

if ! grep -A2 'android:name=".StageFBGateActivity"' "${MANIFEST_PATH}" | \
  grep --fixed-strings 'android:exported="false"' >/dev/null 2>&1; then
  fail "Stage F-B gate activity must remain non-exported"
fi

for path in "${ACTIVITY_PATH}" "${STATUS_CLIENT_PATH}"; do
  for pattern in \
    "${SUBMISSION_METHOD}" \
    "${MAINNET_RPC}" \
    "ClipboardManager" \
    "signTransaction" \
    "signMessage" \
    "signBytes" \
    "createLockedDevnetVault" \
    "rekeyLockedDevnetVault" \
    "exportLockedVaultRecoveryWords" \
    "beginAttempt("; do
    if grep -F "${pattern}" "${path}" >/dev/null; then
      fail "Stage F-B read-only runtime contains forbidden capability: ${pattern} in ${path}"
    fi
  done
done

if grep -F 'NativeBridge.' "${ACTIVITY_PATH}" | grep -v -F 'NativeBridge.devnetBlockHeight()' >/dev/null; then
  fail "Stage F-B gate may call only the native read-only block-height capability"
fi

if grep -F 'NativeBridge.' "${STATUS_CLIENT_PATH}" | grep -v -F 'NativeBridge.rpcEndpoint()' >/dev/null; then
  fail "Stage F-B status client may call only the native read-only endpoint accessor"
fi

echo "Stage F-B guarded read-only resolution audit passed"
