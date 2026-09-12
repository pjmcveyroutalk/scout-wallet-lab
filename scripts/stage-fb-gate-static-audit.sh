#!/usr/bin/env bash
set -euo pipefail

readonly ACTIVITY_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBGateActivity.kt"
readonly GUARD_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBAttemptGuard.kt"
readonly HUB_PATH="android/app/src/main/java/com/routalk/scoutoperator/OperatorHubActivity.kt"
readonly MANIFEST_PATH="android/app/src/main/AndroidManifest.xml"
readonly MAINNET_RPC="https://api.""mainnet-beta.solana.com"
readonly SUBMISSION_METHOD="send""Transaction"

fail() {
  echo "STAGE F-B GATE AUDIT FAILED: $1" >&2
  exit 1
}

for path in "${ACTIVITY_PATH}" "${GUARD_PATH}" "${HUB_PATH}" "${MANIFEST_PATH}"; do
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

grep -F 'StageFBAttemptGuard(this).load()' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate must expose only the persisted one-attempt guard read-only state"

grep -F 'ONE-ATTEMPT GUARD — CLEAR' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate clear guard state is missing"

grep -F 'ONE-ATTEMPT GUARD — CORRUPT / FAIL CLOSED' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate corrupt guard fail-closed state is missing"

grep -F 'ONE-ATTEMPT GUARD — RECORD PRESENT' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate recorded-attempt state is missing"

grep -F 'Candidate fingerprint SHA-256: ' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate must expose the persisted public candidate fingerprint"

grep -F 'loaded.record.candidateFingerprintSha256' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate must read the persisted candidate fingerprint without mutation"

grep -F 'Review receipt SHA-256: ' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate must expose the persisted public review receipt"

grep -F 'loaded.record.reviewReceiptSha256' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate must read the persisted review receipt without mutation"

grep -F 'BINDING OBSERVATION ONLY — EXECUTION NOT AUTHORIZED' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B gate must state that binding observation does not authorize execution"

grep -F 'StageFBGateActivity::class.java' "${HUB_PATH}" >/dev/null || \
  fail "Stage F-B operator hub entry is missing"

if ! grep -A2 'android:name=".StageFBGateActivity"' "${MANIFEST_PATH}" | \
  grep --fixed-strings 'android:exported="false"' >/dev/null 2>&1; then
  fail "Stage F-B gate activity must remain non-exported"
fi

for pattern in \
  "${SUBMISSION_METHOD}" \
  "${MAINNET_RPC}" \
  "NativeBridge." \
  "ClipboardManager" \
  "signTransaction" \
  "signMessage" \
  "signBytes" \
  "createLockedDevnetVault" \
  "rekeyLockedDevnetVault" \
  "exportLockedVaultRecoveryWords" \
  "beginAttempt(" \
  "updateFromResolution("; do
  if grep -F "${pattern}" "${ACTIVITY_PATH}" >/dev/null; then
    fail "Stage F-B guarded screen contains forbidden capability or guard mutation: ${pattern}"
  fi
done

echo "Stage F-B guarded operator gate audit passed"
