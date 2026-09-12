#!/usr/bin/env bash
set -euo pipefail

readonly REPO_ROOT="$(git rev-parse --show-toplevel)"
readonly REVIEW_RECEIPT_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageFBCandidateReviewReceipt.kt"

cd "${REPO_ROOT}"

fail() {
  echo "STAGE F-B CANDIDATE REVIEW RECEIPT AUDIT FAILED: $1" >&2
  exit 1
}

require_text() {
  local pattern="$1"
  local description="$2"

  if grep --fixed-strings --quiet -- "${pattern}" "${REVIEW_RECEIPT_PATH}"; then
    :
  else
    fail "${description}"
  fi
}

forbid_text() {
  local pattern="$1"
  local description="$2"

  if grep --fixed-strings --quiet -- "${pattern}" "${REVIEW_RECEIPT_PATH}"; then
    fail "${description}"
  fi
}

[[ -f "${REVIEW_RECEIPT_PATH}" ]] || fail "candidate review receipt model is missing"

require_text \
  "scout_stage_fb_candidate_review_receipt_v1" \
  "review receipt version marker is missing"

require_text \
  "StageFBCandidateFingerprint.derive(metadata)" \
  "review receipt must re-derive and bind the candidate fingerprint"

require_text \
  "execution_authorized=false" \
  "review receipt must explicitly remain non-authorizing"

require_text \
  "ledger_write_armed=false" \
  "review receipt must explicitly remain unarmed"

require_text \
  "review_scope=public_metadata_only" \
  "review receipt must remain limited to public metadata"

forbid_text \
  "NativeBridge" \
  "review receipt must not call the native bridge"

forbid_text \
  "SharedPreferences" \
  "review receipt must not persist state"

forbid_text \
  "android.content.Context" \
  "review receipt must remain a pure model with no Android context"

forbid_text \
  "java.net" \
  "review receipt must not contain networking"

forbid_text \
  "okhttp" \
  "review receipt must not contain an HTTP client"

forbid_text \
  "api.mainnet-beta.solana.com" \
  "review receipt must not contain a Mainnet endpoint"

echo "Stage F-B candidate review receipt audit passed."
