#!/usr/bin/env bash
set -euo pipefail

readonly TEST_PATH="android/app/src/test/java/com/routalk/scoutoperator/StageFBPublicBindingCheckTest.kt"
readonly BUILD_PATH="android/app/build.gradle.kts"

fail() {
  echo "STAGE F-B BINDING VERIFIER TEST AUDIT FAILED: $1" >&2
  exit 1
}

for path in "${TEST_PATH}" "${BUILD_PATH}"; do
  [[ -f "${path}" ]] || fail "required verifier-test file is missing: ${path}"
done

for test_name in \
  'exactReviewedCandidateIsBound' \
  'mismatchedPersistedCandidateFingerprintFailsClosed' \
  'mismatchedPersistedReviewReceiptFailsClosed' \
  'mismatchedPersistedSignatureFailsClosed' \
  'mismatchedPersistedLastValidBlockHeightFailsClosed' \
  'missingAttemptStartedFailsClosed' \
  'malformedClaimedReviewReceiptFailsClosed' \
  'mismatchedClaimedCandidateFingerprintFailsClosed'; do
  grep -F "fun ${test_name}()" "${TEST_PATH}" >/dev/null || \
    fail "required binding-verifier case is missing: ${test_name}"
done

grep -F 'testImplementation("junit:junit:4.13.2")' "${BUILD_PATH}" >/dev/null || \
  fail "JUnit dependency is missing"

grep -F 'dependsOn("testDebugUnitTest")' "${BUILD_PATH}" >/dev/null || \
  fail "assembleDebug must depend on Stage F-B verifier unit tests"

readonly SUBMISSION_METHOD="send""Transaction"

for pattern in \
  'NativeBridge' \
  "${SUBMISSION_METHOD}" \
  'api.mainnet-beta.solana.com' \
  'passphrase' \
  'recovery' \
  'seed' \
  'privateKey' \
  'signedWire' \
  'wireBytes'; do
  if grep -F "${pattern}" "${TEST_PATH}" >/dev/null; then
    fail "verifier tests contain forbidden capability or secret-bearing surface: ${pattern}"
  fi
done

echo "Stage F-B binding verifier test audit passed"
