#!/usr/bin/env bash
set -euo pipefail

readonly REPO_ROOT="$(git rev-parse --show-toplevel)"
readonly BINDING_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBPublicBindingCheck.kt"

cd "${REPO_ROOT}"

fail() {
  echo "STAGE F-B PUBLIC BINDING CHECK AUDIT FAILED: $1" >&2
  exit 1
}

require_text() {
  local pattern="$1"
  local description="$2"

  if grep --fixed-strings --quiet -- "${pattern}" "${BINDING_PATH}"; then
    :
  else
    fail "${description}"
  fi
}

forbid_text() {
  local pattern="$1"
  local description="$2"

  if grep --fixed-strings --quiet -- "${pattern}" "${BINDING_PATH}"; then
    fail "${description}"
  fi
}

[[ -f "${BINDING_PATH}" ]] || fail "public binding check model is missing"

require_text \
  "StageFBCandidateReviewReceipt.create(" \
  "binding check must re-derive the candidate review receipt"

require_text \
  "guardRecord.candidateFingerprintSha256 != receipt.candidateFingerprintSha256" \
  "binding check must require the persisted candidate fingerprint to match the re-derived fingerprint"

require_text \
  "guardRecord.reviewReceiptSha256 != receipt.reviewReceiptSha256" \
  "binding check must require the persisted review receipt to match the re-derived receipt"

require_text \
  "guardRecord.expectedSignature != metadata.expectedSignature" \
  "binding check must bind the guard to the exact public signature"

require_text \
  "guardRecord.lastValidBlockHeight != metadata.lastValidBlockHeight" \
  "binding check must bind the guard to the candidate expiry boundary"

require_text \
  "guardRecord.attemptStarted" \
  "binding check must require an existing one-attempt guard record"

for pattern in \
  'SharedPreferences' \
  'android.content.Context' \
  'NativeBridge' \
  'beginAttempt(' \
  'updateFromResolution(' \
  'java.net' \
  'okhttp' \
  'api.mainnet-beta.solana.com' \
  'passphrase' \
  'recovery' \
  'seed' \
  'privateKey' \
  'signedWire' \
  'wireBytes'; do
  forbid_text "${pattern}" "binding check contains forbidden mutation, network, or secret-bearing surface: ${pattern}"
done

echo "Stage F-B public binding check audit passed."
