#!/usr/bin/env bash
set -euo pipefail

readonly GUARD_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBAttemptGuard.kt"
readonly RESOLUTION_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBReadOnlyResolution.kt"

fail() {
  echo "STAGE F-B ATTEMPT GUARD AUDIT FAILED: $1" >&2
  exit 1
}

for path in "${GUARD_PATH}" "${RESOLUTION_PATH}"; do
  [[ -f "${path}" ]] || fail "required Stage F-B readiness file is missing: ${path}"
done

grep -F 'candidate_fingerprint_sha256' "${GUARD_PATH}" >/dev/null || \
  fail "candidate-fingerprint persistence key is missing"

grep -F 'review_receipt_sha256' "${GUARD_PATH}" >/dev/null || \
  fail "review-receipt persistence key is missing"

grep -F 'expected_public_signature' "${GUARD_PATH}" >/dev/null || \
  fail "public expected-signature persistence key is missing"

grep -F 'attempt_started' "${GUARD_PATH}" >/dev/null || \
  fail "attempt-started persistence key is missing"

grep -F 'last_valid_block_height' "${GUARD_PATH}" >/dev/null || \
  fail "blockhash-expiry persistence key is missing"

grep -F 'final_public_status' "${GUARD_PATH}" >/dev/null || \
  fail "public resolution-status persistence key is missing"

grep -F 'isLowercaseSha256(candidateFingerprintSha256)' "${GUARD_PATH}" >/dev/null || \
  fail "candidate fingerprint must be validated before persistence"

grep -F 'isLowercaseSha256(reviewReceiptSha256)' "${GUARD_PATH}" >/dev/null || \
  fail "review receipt must be validated before persistence"

grep -F '.putString(KEY_CANDIDATE_FINGERPRINT_SHA256, candidateFingerprintSha256)' "${GUARD_PATH}" >/dev/null || \
  fail "candidate fingerprint must be persisted with the one-attempt guard"

grep -F '.putString(KEY_REVIEW_RECEIPT_SHA256, reviewReceiptSha256)' "${GUARD_PATH}" >/dev/null || \
  fail "review receipt must be persisted with the one-attempt guard"

grep -F '.commit()' "${GUARD_PATH}" >/dev/null || \
  fail "guard persistence must use synchronous commit before later capability wiring"

grep -F 'STARTED_AND_PERSISTED' "${GUARD_PATH}" >/dev/null || \
  fail "guard must verify durable attempt-start persistence"

grep -F 'ALREADY_GUARDED' "${GUARD_PATH}" >/dev/null || \
  fail "guard must refuse a second attempt after any persisted attempt record"

grep -F 'CORRUPT_GUARD' "${GUARD_PATH}" >/dev/null || \
  fail "corrupt persisted guard state must fail closed"

grep -F 'StageFBReadOnlyResolution.Resolution' "${GUARD_PATH}" >/dev/null || \
  fail "guard must consume only the read-only resolution state model"

for pattern in \
  'NativeBridge' \
  'ClipboardManager' \
  'ByteArray' \
  'passphrase' \
  'recovery' \
  'seed' \
  'privateKey' \
  'signedWire' \
  'wireBytes' \
  'http://' \
  'https://' \
  '.clear()' \
  '.remove('; do
  if grep -F "${pattern}" "${GUARD_PATH}" >/dev/null; then
    fail "attempt guard contains forbidden capability or secret-bearing surface: ${pattern}"
  fi
done

echo "Stage F-B persistent one-attempt guard audit passed"
