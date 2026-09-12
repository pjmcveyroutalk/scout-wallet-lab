#!/usr/bin/env bash
set -euo pipefail

readonly CORE_PATH="crates/wallet-engine/src/stage_f_presubmit.rs"
readonly SIGNING_MODULE_PATH="crates/wallet-engine/src/recovery_words.rs"
readonly NATIVE_LIB_PATH="android/native/src/lib.rs"
readonly NATIVE_PATH="android/native/src/stage_f_presubmit.rs"
readonly BRIDGE_PATH="android/app/src/main/java/com/routalk/scoutoperator/NativeBridge.kt"
readonly ACTIVITY_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFPresubmitActivity.kt"
readonly HUB_PATH="android/app/src/main/java/com/routalk/scoutoperator/OperatorHubActivity.kt"
readonly MANIFEST_PATH="android/app/src/main/AndroidManifest.xml"
readonly SUBMISSION_METHOD="send""Transaction"
readonly MAINNET_RPC="https://api.""mainnet-beta.solana.com"

fail() {
  echo "STAGE F PRESUBMIT AUDIT FAILED: $1" >&2
  exit 1
}

for path in \
  "${CORE_PATH}" \
  "${SIGNING_MODULE_PATH}" \
  "${NATIVE_LIB_PATH}" \
  "${NATIVE_PATH}" \
  "${BRIDGE_PATH}" \
  "${ACTIVITY_PATH}" \
  "${HUB_PATH}" \
  "${MANIFEST_PATH}"; do
  [[ -f "${path}" ]] || fail "required Stage F-A file is missing: ${path}"
done

grep -F 'pub mod stage_f_presubmit;' "${SIGNING_MODULE_PATH}" >/dev/null || \
  fail "Stage F-A wallet-engine module wiring is missing"

grep -F 'mod stage_f_presubmit;' "${NATIVE_LIB_PATH}" >/dev/null || \
  fail "Stage F-A native module wiring is missing"

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

grep -F 'prepareStageFDevnetCandidate' "${BRIDGE_PATH}" >/dev/null || \
  fail "narrow Android Stage F-A prepare request is missing"

grep -F 'discardStageFDevnetCandidate' "${BRIDGE_PATH}" >/dev/null || \
  fail "narrow Android Stage F-A discard request is missing"

grep -F 'NativeBridge_prepareStageFDevnetCandidate' "${NATIVE_PATH}" >/dev/null || \
  fail "narrow JNI Stage F-A prepare export is missing"

grep -F 'NativeBridge_discardStageFDevnetCandidate' "${NATIVE_PATH}" >/dev/null || \
  fail "narrow JNI Stage F-A discard export is missing"

grep -F 'PRESUBMIT ONLY — NO LEDGER SUBMISSION' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-A no-ledger-submission statement is missing"

grep -F 'MAINNET — DISABLED' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-A Mainnet safety statement is missing"

grep -F 'ARBITRARY SIGNING — DISABLED' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-A arbitrary-signing safety statement is missing"

grep -F 'NativeBridge.prepareStageFDevnetCandidate(' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-A activity must use only the narrow presubmit prepare request"

grep -F 'NativeBridge.discardStageFDevnetCandidate(' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-A activity must provide in-memory candidate discard"

grep -F 'StageFPresubmitActivity::class.java' "${HUB_PATH}" >/dev/null || \
  fail "Stage F-A operator hub entry is missing"

if ! grep -A2 'android:name=".StageFPresubmitActivity"' "${MANIFEST_PATH}" | \
  grep --fixed-strings 'android:exported="false"' >/dev/null 2>&1; then
  fail "Stage F-A activity must remain non-exported"
fi

for path in "${CORE_PATH}" "${NATIVE_PATH}" "${BRIDGE_PATH}" "${ACTIVITY_PATH}"; do
  if grep -F "${SUBMISSION_METHOD}" "${path}" >/dev/null; then
    fail "ledger submission must remain absent from Stage F-A: ${path}"
  fi

  if grep -F "${MAINNET_RPC}" "${path}" >/dev/null; then
    fail "Mainnet RPC must remain absent from Stage F-A: ${path}"
  fi
done

if grep -E 'signTransaction|signMessage|signBytes|ClipboardManager|createLockedDevnetVault|rekeyLockedDevnetVault|exportLockedVaultRecoveryWords' "${ACTIVITY_PATH}" >/dev/null; then
  fail "generic signing, wallet mutation, recovery export, or clipboard surface escaped into Stage F-A"
fi

rustfmt +1.80.0 --edition 2021 --check "${CORE_PATH}" "${NATIVE_PATH}"

echo "Stage F presubmit static audit passed"
