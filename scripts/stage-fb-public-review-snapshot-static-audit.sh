#!/usr/bin/env bash
set -euo pipefail

readonly SNAPSHOT_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBPublicReviewSnapshot.kt"
readonly TEST_PATH="android/app/src/test/java/com/routalk/scoutoperator/StageFBPublicReviewSnapshotTest.kt"
readonly SUBMISSION_METHOD="send""Transaction"
readonly MAINNET_RPC="https://api.""mainnet-beta.solana.com"

fail() {
  echo "STAGE F-B PUBLIC REVIEW SNAPSHOT AUDIT FAILED: $1" >&2
  exit 1
}

[[ -f "${SNAPSHOT_PATH}" ]] || fail "public review snapshot contract is missing"
[[ -f "${TEST_PATH}" ]] || fail "public review snapshot tests are missing"

grep -F 'data class Snapshot(' "${SNAPSHOT_PATH}" >/dev/null || \
  fail "immutable public snapshot model is missing"
grep -F 'fun createFromPublicPresubmit(' "${SNAPSHOT_PATH}" >/dev/null || \
  fail "token-free public presubmit handoff builder is missing"
grep -F 'StageFBPublicSignatureCodec.fromLowercaseHex(signatureHex)' "${SNAPSHOT_PATH}" >/dev/null || \
  fail "public signature conversion is missing"
grep -F 'StageFBCandidateFingerprint.derive(candidate.metadata)' "${SNAPSHOT_PATH}" >/dev/null || \
  fail "candidate fingerprint must be re-derived"
grep -F 'StageFBCandidateReviewReceipt.create(' "${SNAPSHOT_PATH}" >/dev/null || \
  fail "review receipt must be re-derived"
grep -F 'fingerprint.fingerprintSha256 != candidate.candidateFingerprintSha256' "${SNAPSHOT_PATH}" >/dev/null || \
  fail "candidate fingerprint mismatch must fail closed"
grep -F 'receipt.reviewReceiptSha256 != candidate.reviewReceiptSha256' "${SNAPSHOT_PATH}" >/dev/null || \
  fail "review receipt mismatch must fail closed"
grep -F 'candidate.unitsConsumed <= 0L' "${SNAPSHOT_PATH}" >/dev/null || \
  fail "invalid simulation units must fail closed"

for test_name in \
  exactPreparedCandidateProducesPublicSnapshot \
  publicPresubmitFieldsProduceSameSnapshotWithoutCandidateToken \
  malformedPublicSignatureHexFailsClosed \
  tamperedFingerprintFailsClosed \
  tamperedReviewReceiptFailsClosed \
  invalidSimulationUnitsFailClosed; do
  grep -F "fun ${test_name}()" "${TEST_PATH}" >/dev/null || \
    fail "required test is missing: ${test_name}"
done

for pattern in \
  'tokenHex' \
  'candidateToken' \
  'NativeBridge' \
  'SharedPreferences' \
  'android.content.Context' \
  'passphrase' \
  'recovery' \
  'privateKey' \
  'signedWire' \
  'wireBytes' \
  "${SUBMISSION_METHOD}" \
  "${MAINNET_RPC}"; do
  if grep -F "${pattern}" "${SNAPSHOT_PATH}" >/dev/null; then
    fail "non-public capability escaped into public review snapshot: ${pattern}"
  fi
done

echo "Stage F-B public review snapshot audit passed"
