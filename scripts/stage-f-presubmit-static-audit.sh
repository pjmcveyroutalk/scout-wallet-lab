#!/usr/bin/env bash
set -euo pipefail

readonly CORE_PATH="crates/wallet-engine/src/stage_f_presubmit.rs"
readonly SIGNING_MODULE_PATH="crates/wallet-engine/src/recovery_words.rs"
readonly NATIVE_LIB_PATH="android/native/src/lib.rs"
readonly NATIVE_PATH="android/native/src/stage_f_presubmit.rs"
readonly BRIDGE_PATH="android/app/src/main/java/com/routalk/scoutoperator/NativeBridge.kt"
readonly ACTIVITY_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFPresubmitActivity.kt"
readonly SNAPSHOT_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBPublicReviewSnapshot.kt"
readonly HUB_PATH="android/app/src/main/java/com/routalk/scoutoperator/OperatorHubActivity.kt"
readonly MANIFEST_PATH="android/app/src/main/AndroidManifest.xml"
readonly SUBMISSION_METHOD="send""Transaction"
readonly MAINNET_RPC="https://api.""mainnet-beta.solana.com"

fail() {
  echo "STAGE F AUDIT FAILED: $1" >&2
  exit 1
}

for path in \
  "${CORE_PATH}" \
  "${SIGNING_MODULE_PATH}" \
  "${NATIVE_LIB_PATH}" \
  "${NATIVE_PATH}" \
  "${BRIDGE_PATH}" \
  "${ACTIVITY_PATH}" \
  "${SNAPSHOT_PATH}" \
  "${HUB_PATH}" \
  "${MANIFEST_PATH}"; do
  [[ -f "${path}" ]] || fail "required Stage F file is missing: ${path}"
done

grep -F 'pub mod stage_f_presubmit;' "${SIGNING_MODULE_PATH}" >/dev/null || \
  fail "Stage F wallet-engine module wiring is missing"

grep -F 'mod stage_f_presubmit;' "${NATIVE_LIB_PATH}" >/dev/null || \
  fail "Stage F native module wiring is missing"

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

grep -F 'CandidateLifecycleInvalid' "${CORE_PATH}" >/dev/null || \
  fail "signed candidate lifecycle guard is missing"

grep -F 'CandidateExpired' "${CORE_PATH}" >/dev/null || \
  fail "expired-candidate fail-closed state is missing"

grep -F 'take_prepared_candidate_wire_for_submission(' "${CORE_PATH}" >/dev/null || \
  fail "consume-once Stage F-B candidate handoff is missing"

grep -F 'lifecycle_state: transaction.ledger().state()' "${CORE_PATH}" >/dev/null || \
  fail "signed transaction lifecycle state is not preserved with the candidate"

grep -F 'prepareStageFDevnetCandidate' "${BRIDGE_PATH}" >/dev/null || \
  fail "narrow Android Stage F-A prepare request is missing"

grep -F 'discardStageFDevnetCandidate' "${BRIDGE_PATH}" >/dev/null || \
  fail "narrow Android Stage F candidate discard request is missing"

grep -F 'submitStageFDevnetCandidateOnce' "${BRIDGE_PATH}" >/dev/null || \
  fail "narrow Android Stage F-B one-shot submit request is missing"

grep -F 'NativeBridge_prepareStageFDevnetCandidate' "${NATIVE_PATH}" >/dev/null || \
  fail "narrow JNI Stage F-A prepare export is missing"

grep -F 'NativeBridge_discardStageFDevnetCandidate' "${NATIVE_PATH}" >/dev/null || \
  fail "narrow JNI Stage F candidate discard export is missing"

grep -F 'NativeBridge_submitStageFDevnetCandidateOnce' "${NATIVE_PATH}" >/dev/null || \
  fail "narrow JNI Stage F-B one-shot submit export is missing"

grep -F 'method: "sendTransaction"' "${NATIVE_PATH}" >/dev/null || \
  fail "Stage F-B Devnet ledger-write RPC is missing"

grep -F 'max_retries: 0' "${NATIVE_PATH}" >/dev/null || \
  fail "Stage F-B RPC retries must remain disabled"

grep -F 'skip_preflight: false' "${NATIVE_PATH}" >/dev/null || \
  fail "Stage F-B must retain RPC preflight"

grep -F 'preflight_commitment: "confirmed"' "${NATIVE_PATH}" >/dev/null || \
  fail "Stage F-B preflight commitment must remain confirmed"

grep -F 'Cluster::Devnet.rpc_url()' "${NATIVE_PATH}" >/dev/null || \
  fail "Stage F-B submission must use the pinned Devnet cluster endpoint"

grep -F 'transport-ambiguous-terminal' "${NATIVE_PATH}" >/dev/null || \
  fail "ambiguous Stage F-B transport outcome must remain terminal"

grep -F 'STAGE F-B PHYSICAL SEND — NOT ARMED' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B physical-send not-armed statement is missing"

grep -F 'STAGE_FB_PHYSICAL_SEND_ARMED = false' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B physical send must remain hard-disabled pending separate operator authorization"

grep -F 'MAINNET — DISABLED' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F Mainnet safety statement is missing"

grep -F 'ARBITRARY SIGNING — DISABLED' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F arbitrary-signing safety statement is missing"

grep -F 'NativeBridge.prepareStageFDevnetCandidate(' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F activity must use the narrow presubmit prepare request"

grep -F 'NativeBridge.discardStageFDevnetCandidate(' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F activity must provide in-memory candidate discard"

grep -F 'NativeBridge.submitStageFDevnetCandidateOnce(' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F activity must use only the narrow one-shot submit request"

grep -F 'guard.beginAttempt(' "${ACTIVITY_PATH}" >/dev/null || \
  fail "one-attempt guard must be persisted before the Stage F-B ledger write"

grep -F 'StageFBPreArmingEligibility.evaluate(' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B must re-check reviewed-candidate eligibility after guard persistence"

grep -F 'DO NOT RETRY' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-B terminal no-retry operator messaging is missing"

grep -F 'StageFBPublicReviewSnapshot.createFromPublicPresubmit(' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-A must derive the token-free Stage F-B public review snapshot"

grep -F 'PRESUBMIT FAILED — STAGE F-B PUBLIC REVIEW SNAPSHOT INVALID' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-A must fail closed when the public review snapshot is invalid"

grep -F 'STAGE F-B PUBLIC REVIEW SNAPSHOT — VERIFIED' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-A must visibly report verified public review binding"

grep -F 'EXECUTION AUTHORIZED: NO' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F-A public review must not itself authorize execution"

grep -F 'preparedPublicReviewSnapshot = null' "${ACTIVITY_PATH}" >/dev/null || \
  fail "Stage F must clear the in-memory public review snapshot"

grep -F 'fun createFromPublicPresubmit(' "${SNAPSHOT_PATH}" >/dev/null || \
  fail "token-free public presubmit snapshot builder is missing"

grep -F 'StageFPresubmitActivity::class.java' "${HUB_PATH}" >/dev/null || \
  fail "Stage F operator hub entry is missing"

if ! grep -A2 'android:name=".StageFPresubmitActivity"' "${MANIFEST_PATH}" | \
  grep --fixed-strings 'android:exported="false"' >/dev/null 2>&1; then
  fail "Stage F activity must remain non-exported"
fi

for path in "${CORE_PATH}" "${BRIDGE_PATH}" "${ACTIVITY_PATH}"; do
  if grep -F "${SUBMISSION_METHOD}" "${path}" >/dev/null; then
    fail "raw ledger-write RPC method escaped the narrow native Stage F-B boundary: ${path}"
  fi
done

for path in "${CORE_PATH}" "${NATIVE_PATH}" "${BRIDGE_PATH}" "${ACTIVITY_PATH}"; do
  if grep -F "${MAINNET_RPC}" "${path}" >/dev/null; then
    fail "Mainnet RPC must remain absent from Stage F: ${path}"
  fi
done

if grep -E 'signTransaction|signMessage|signBytes|ClipboardManager|createLockedDevnetVault|rekeyLockedDevnetVault|exportLockedVaultRecoveryWords|SharedPreferences' "${ACTIVITY_PATH}" >/dev/null; then
  fail "generic signing, wallet mutation, recovery export, clipboard, or direct persistence escaped into Stage F"
fi

rustfmt +1.80.0 --edition 2021 --check "${CORE_PATH}" "${NATIVE_PATH}"

echo "Stage F static audit passed"
