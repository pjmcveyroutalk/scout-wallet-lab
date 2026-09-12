#!/usr/bin/env bash
set -euo pipefail

readonly FINGERPRINT_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBCandidateFingerprint.kt"

fail() {
  echo "STAGE F-B CANDIDATE FINGERPRINT AUDIT FAILED: $1" >&2
  exit 1
}

[[ -f "${FINGERPRINT_PATH}" ]] || fail "candidate fingerprint model is missing"

grep -F 'MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr' "${FINGERPRINT_PATH}" >/dev/null || \
  fail "fixed Memo program is missing"

grep -F 'scout-stage-f-devnet-submission-proof-v1' "${FINGERPRINT_PATH}" >/dev/null || \
  fail "fixed Stage F payload is missing"

grep -F 'MAX_FEE_LAMPORTS = 10_000L' "${FINGERPRINT_PATH}" >/dev/null || \
  fail "10,000 lamport fee ceiling is missing"

grep -F 'MIN_REMAINING_BALANCE_LAMPORTS = 1_000_000L' "${FINGERPRINT_PATH}" >/dev/null || \
  fail "1,000,000 lamport remaining-balance floor is missing"

grep -F 'remainingBalanceLamports != metadata.balanceLamports - metadata.feeLamports' "${FINGERPRINT_PATH}" >/dev/null || \
  fail "balance arithmetic equality check is missing"

grep -F 'MessageDigest' "${FINGERPRINT_PATH}" >/dev/null || \
  fail "SHA-256 digest implementation is missing"

grep -F 'getInstance("SHA-256")' "${FINGERPRINT_PATH}" >/dev/null || \
  fail "SHA-256 algorithm pin is missing"

grep -F 'canonicalReviewText' "${FINGERPRINT_PATH}" >/dev/null || \
  fail "canonical public review text is missing"

for pattern in \
  'NativeBridge' \
  'signTransaction' \
  'signMessage' \
  'signBytes' \
  'passphrase' \
  'recovery' \
  'seed' \
  'privateKey' \
  'signedWire' \
  'wireBytes' \
  'ClipboardManager' \
  'http://' \
  'https://'; do
  if grep -F "${pattern}" "${FINGERPRINT_PATH}" >/dev/null; then
    fail "candidate fingerprint model contains forbidden capability or secret surface: ${pattern}"
  fi
done

# The repository-wide security tripwire remains authoritative for transaction-submission primitives.
echo "Stage F-B candidate fingerprint audit passed"
