#!/usr/bin/env bash
set -euo pipefail

readonly REPO_ROOT="$(git rev-parse --show-toplevel)"
readonly EXPORTER_PATH="crates/wallet-engine/src/bin/export_observability.rs"
readonly SIGNING_COORDINATOR_PATH="crates/wallet-engine/src/devnet_signing_coordinator.rs"
readonly STAGE_E_CORE_PATH="crates/wallet-engine/src/stage_e_preflight.rs"
readonly SIGNING_MODULE_PATH="crates/wallet-engine/src/recovery_words.rs"
readonly ANDROID_NATIVE_PATH="android/native/src/lib.rs"
readonly STAGE_E_NATIVE_PATH="android/native/src/stage_e_preflight.rs"
readonly ANDROID_BRIDGE_PATH="android/app/src/main/java/com/routalk/scoutoperator/NativeBridge.kt"
readonly CREDENTIAL_RECOVERY_NATIVE_PATH="android/native/src/credential_recovery.rs"
readonly CREDENTIAL_RECOVERY_ACTIVITY_PATH="android/app/src/main/java/com/routalk/scoutoperator/CredentialRecoveryActivity.kt"
readonly CREDENTIAL_REKEY_NATIVE_PATH="android/native/src/credential_rekey.rs"
readonly CREDENTIAL_REKEY_ACTIVITY_PATH="android/app/src/main/java/com/routalk/scoutoperator/CredentialRekeyActivity.kt"
readonly STAGE_D_ACTIVITY_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageDProofActivity.kt"
readonly STAGE_E_ACTIVITY_PATH="android/app/src/main/java/com/routalk/scoutoperator/StageEPreflightActivity.kt"
readonly OPERATOR_HUB_ACTIVITY_PATH="android/app/src/main/java/com/routalk/scoutoperator/OperatorHubActivity.kt"
readonly ANDROID_MANIFEST_PATH="android/app/src/main/AndroidManifest.xml"
readonly SIGNING_DESIGN_DOC="docs/DEVNET_SIGNING_BOUNDARY_V1.md"

cd "${REPO_ROOT}"

fail() {
  echo "SECURITY TRIPWIRE FAILED: $1" >&2
  exit 1
}

assert_not_tracked() {
  local path="$1"

  if git ls-files --error-unmatch "${path}" >/dev/null 2>&1; then
    fail "generated or secret-bearing artifact is tracked: ${path}"
  fi
}

assert_absent_in_source() {
  local pattern="$1"
  local description="$2"

  if git grep \
    --line-number \
    --fixed-strings \
    -- "${pattern}" \
    ':!scripts/security-tripwires.sh' \
    ':!README.md' \
    ":!${SIGNING_DESIGN_DOC}" \
    >/dev/null 2>&1; then
    fail "${description}"
  fi
}

assert_absent_in_path() {
  local pattern="$1"
  local path="$2"
  local description="$3"

  if git grep \
    --line-number \
    --fixed-strings \
    -- "${pattern}" \
    -- "${path}" \
    >/dev/null 2>&1; then
    fail "${description}"
  fi
}

assert_present_in_path() {
  local pattern="$1"
  local path="$2"
  local description="$3"

  if git grep \
    --line-number \
    --fixed-strings \
    -- "${pattern}" \
    -- "${path}" \
    >/dev/null 2>&1; then
    :
  else
    fail "${description}"
  fi
}

assert_passphrase_boundary() {
  local matches

  matches="$(
    git grep \
      --line-number \
      --fixed-strings \
      -- "SCOUT_WALLET_PASSPHRASE" \
      ':!scripts/security-tripwires.sh' \
      ':!README.md' \
      ":!${SIGNING_DESIGN_DOC}" \
      2>/dev/null || true
  )"

  if [[ -z "${matches}" ]]; then
    fail "local exporter passphrase boundary is missing"
  fi

  while IFS= read -r match; do
    if [[ "${match}" != "${EXPORTER_PATH}:"* ]]; then
      fail "wallet passphrase environment variable escaped the local exporter boundary"
    fi
  done <<< "${matches}"
}

echo "Checking generated observability artifacts..."

assert_not_tracked "dashboard/wallet-observability.json"
assert_not_tracked "dashboard/.wallet-observability.json.tmp"

echo "Checking network boundary..."

assert_absent_in_source \
  "https://api.mainnet-beta.solana.com" \
  "mainnet RPC endpoint must remain absent"

assert_absent_in_source \
  "sendTransaction" \
  "transaction submission must remain disabled until the Devnet submission gate is explicitly opened"

echo "Checking signer boundary..."

assert_absent_in_source \
  "solana-keypair" \
  "generic Solana keypair dependency is forbidden"

assert_absent_in_source \
  "solana_keypair" \
  "generic Solana keypair API is forbidden"

assert_absent_in_source \
  "get_keypair" \
  "raw keypair access API is forbidden"

assert_absent_in_source \
  "sign_arbitrary" \
  "arbitrary signing API is forbidden"

assert_absent_in_path \
  "signTransaction" \
  "${ANDROID_BRIDGE_PATH}" \
  "generic Android transaction signing API is forbidden"

assert_absent_in_path \
  "signMessage" \
  "${ANDROID_BRIDGE_PATH}" \
  "Android arbitrary-message signing API is forbidden"

assert_absent_in_path \
  "signBytes" \
  "${ANDROID_BRIDGE_PATH}" \
  "Android arbitrary-byte signing API is forbidden"

assert_absent_in_path \
  "NativeBridge_signTransaction" \
  "${ANDROID_NATIVE_PATH}" \
  "generic JNI transaction signing export is forbidden"

assert_absent_in_path \
  "NativeBridge_signMessage" \
  "${ANDROID_NATIVE_PATH}" \
  "JNI arbitrary-message signing export is forbidden"

assert_absent_in_path \
  "NativeBridge_signBytes" \
  "${ANDROID_NATIVE_PATH}" \
  "JNI arbitrary-byte signing export is forbidden"

assert_present_in_path \
  "pub mod devnet_signing_coordinator;" \
  "${SIGNING_MODULE_PATH}" \
  "Devnet signing coordinator module wiring is missing"

assert_present_in_path \
  "pub fn sign_stage_c_proof(" \
  "${SIGNING_COORDINATOR_PATH}" \
  "fixed Stage C Devnet proof signer is missing"

assert_present_in_path \
  "scout-stage-c-devnet-signing-proof-v1" \
  "${SIGNING_COORDINATOR_PATH}" \
  "fixed Stage C proof payload is missing"

assert_present_in_path \
  "signStageCDevnetProof" \
  "${ANDROID_BRIDGE_PATH}" \
  "narrow Android Stage C signing request is missing"

assert_present_in_path \
  "NativeBridge_signStageCDevnetProof" \
  "${ANDROID_NATIVE_PATH}" \
  "narrow JNI Stage C signing export is missing"

echo "Checking Stage D physical proof boundary..."

assert_present_in_path \
  "NativeBridge.signStageCDevnetProof(" \
  "${STAGE_D_ACTIVITY_PATH}" \
  "Stage D must use only the fixed Stage C signing request"

assert_present_in_path \
  "TRANSACTION SUBMISSION — DISABLED" \
  "${STAGE_D_ACTIVITY_PATH}" \
  "Stage D transaction-submission safety statement is missing"

assert_present_in_path \
  "MAINNET — DISABLED" \
  "${STAGE_D_ACTIVITY_PATH}" \
  "Stage D Mainnet safety statement is missing"

assert_present_in_path \
  "ARBITRARY SIGNING — DISABLED" \
  "${STAGE_D_ACTIVITY_PATH}" \
  "Stage D arbitrary-signing safety statement is missing"

assert_present_in_path \
  "StageDProofActivity::class.java" \
  "${OPERATOR_HUB_ACTIVITY_PATH}" \
  "Stage D operator hub launcher is missing"

assert_absent_in_path \
  "sendTransaction" \
  "${STAGE_D_ACTIVITY_PATH}" \
  "Stage D must never submit a transaction"

assert_absent_in_path \
  "signTransaction" \
  "${STAGE_D_ACTIVITY_PATH}" \
  "Stage D generic transaction signing is forbidden"

assert_absent_in_path \
  "signMessage" \
  "${STAGE_D_ACTIVITY_PATH}" \
  "Stage D arbitrary-message signing is forbidden"

assert_absent_in_path \
  "signBytes" \
  "${STAGE_D_ACTIVITY_PATH}" \
  "Stage D arbitrary-byte signing is forbidden"

assert_absent_in_path \
  "createLockedDevnetVault" \
  "${STAGE_D_ACTIVITY_PATH}" \
  "Stage D must not create or replace the wallet"

assert_absent_in_path \
  "rekeyLockedDevnetVault" \
  "${STAGE_D_ACTIVITY_PATH}" \
  "Stage D must not re-key the wallet"

assert_absent_in_path \
  "exportLockedVaultRecoveryWords" \
  "${STAGE_D_ACTIVITY_PATH}" \
  "Stage D must not export recovery words"

assert_absent_in_path \
  "ClipboardManager" \
  "${STAGE_D_ACTIVITY_PATH}" \
  "Stage D proof metadata must not be copied to the clipboard"

if ! grep -A2 'android:name=".StageDProofActivity"' "${ANDROID_MANIFEST_PATH}" | \
  grep --fixed-strings 'android:exported="false"' >/dev/null 2>&1; then
  fail "Stage D proof activity must remain non-exported"
fi

echo "Checking Stage E simulation-only boundary..."

assert_present_in_path \
  "pub mod stage_e_preflight;" \
  "${SIGNING_MODULE_PATH}" \
  "Stage E simulation preflight module wiring is missing"

assert_present_in_path \
  "scout-stage-e-devnet-simulation-proof-v1" \
  "${STAGE_E_CORE_PATH}" \
  "fixed Stage E simulation payload is missing"

assert_present_in_path \
  "STAGE_E_MAX_FEE_LAMPORTS: u64 = 10_000" \
  "${STAGE_E_CORE_PATH}" \
  "Stage E fixed fee ceiling is missing"

assert_present_in_path \
  'method: "getFeeForMessage"' \
  "${STAGE_E_CORE_PATH}" \
  "Stage E fee preflight RPC is missing"

assert_present_in_path \
  'method: "simulateTransaction"' \
  "${STAGE_E_CORE_PATH}" \
  "Stage E simulation RPC is missing"

assert_present_in_path \
  "simulateStageEDevnetProof" \
  "${ANDROID_BRIDGE_PATH}" \
  "narrow Android Stage E simulation request is missing"

assert_present_in_path \
  "NativeBridge_simulateStageEDevnetProof" \
  "${STAGE_E_NATIVE_PATH}" \
  "narrow JNI Stage E simulation export is missing"

assert_present_in_path \
  "SIMULATION ONLY — NO TRANSACTION SUBMISSION" \
  "${STAGE_E_ACTIVITY_PATH}" \
  "Stage E simulation-only safety statement is missing"

assert_present_in_path \
  "MAINNET — DISABLED" \
  "${STAGE_E_ACTIVITY_PATH}" \
  "Stage E Mainnet safety statement is missing"

assert_present_in_path \
  "ARBITRARY SIGNING — DISABLED" \
  "${STAGE_E_ACTIVITY_PATH}" \
  "Stage E arbitrary-signing safety statement is missing"

assert_present_in_path \
  "StageEPreflightActivity::class.java" \
  "${OPERATOR_HUB_ACTIVITY_PATH}" \
  "Stage E operator hub launcher is missing"

assert_absent_in_path \
  "signStageCDevnetProof" \
  "${STAGE_E_ACTIVITY_PATH}" \
  "Stage E must use only its fixed simulation preflight request"

assert_absent_in_path \
  "createLockedDevnetVault" \
  "${STAGE_E_ACTIVITY_PATH}" \
  "Stage E must not create or replace the wallet"

assert_absent_in_path \
  "rekeyLockedDevnetVault" \
  "${STAGE_E_ACTIVITY_PATH}" \
  "Stage E must not re-key the wallet"

assert_absent_in_path \
  "exportLockedVaultRecoveryWords" \
  "${STAGE_E_ACTIVITY_PATH}" \
  "Stage E must not export recovery words"

assert_absent_in_path \
  "ClipboardManager" \
  "${STAGE_E_ACTIVITY_PATH}" \
  "Stage E proof metadata must not be copied to the clipboard"

if ! grep -A2 'android:name=".StageEPreflightActivity"' "${ANDROID_MANIFEST_PATH}" | \
  grep --fixed-strings 'android:exported="false"' >/dev/null 2>&1; then
  fail "Stage E preflight activity must remain non-exported"
fi

if ! grep -A6 'android:name=".CredentialRecoveryActivity"' "${ANDROID_MANIFEST_PATH}" | \
  grep --fixed-strings 'android.intent.category.LAUNCHER' >/dev/null 2>&1; then
  fail "normal Scout credential-recovery launcher must remain intact for updater compatibility"
fi

echo "Checking credential recovery boundary..."

assert_present_in_path \
  "verifyLockedDevnetPassphrase" \
  "${ANDROID_BRIDGE_PATH}" \
  "narrow Android passphrase verification request is missing"

assert_present_in_path \
  "NativeBridge_verifyLockedDevnetPassphrase" \
  "${CREDENTIAL_RECOVERY_NATIVE_PATH}" \
  "narrow JNI passphrase verification export is missing"

assert_present_in_path \
  "NO SIGNING • NO TRANSACTION • NO VAULT CHANGES" \
  "${CREDENTIAL_RECOVERY_ACTIVITY_PATH}" \
  "credential recovery safety statement is missing"

assert_present_in_path \
  "android:name=\".CredentialRecoveryActivity\"" \
  "${ANDROID_MANIFEST_PATH}" \
  "credential recovery launcher activity is missing"

assert_absent_in_path \
  "signStageCDevnetProof" \
  "${CREDENTIAL_RECOVERY_ACTIVITY_PATH}" \
  "credential recovery activity must not invoke signing"

assert_absent_in_path \
  "createLockedDevnetVault" \
  "${CREDENTIAL_RECOVERY_ACTIVITY_PATH}" \
  "credential recovery activity must not create a wallet"

assert_absent_in_path \
  "saveVault(" \
  "${CREDENTIAL_RECOVERY_ACTIVITY_PATH}" \
  "credential recovery activity must not write or replace the vault"

assert_absent_in_path \
  "export_emergency_recovery_words" \
  "${CREDENTIAL_RECOVERY_NATIVE_PATH}" \
  "credential recovery verification gate must not export recovery words yet"

assert_absent_in_path \
  "restore_from_emergency_recovery_words" \
  "${CREDENTIAL_RECOVERY_NATIVE_PATH}" \
  "credential recovery verification gate must not restore or replace a vault"

echo "Checking credential re-key boundary..."

assert_present_in_path \
  "rekeyLockedDevnetVault" \
  "${ANDROID_BRIDGE_PATH}" \
  "narrow Android passphrase re-key request is missing"

assert_present_in_path \
  "NativeBridge_rekeyLockedDevnetVault" \
  "${CREDENTIAL_REKEY_NATIVE_PATH}" \
  "isolated JNI passphrase re-key export is missing"

assert_present_in_path \
  "restore_from_emergency_recovery_words" \
  "${CREDENTIAL_REKEY_NATIVE_PATH}" \
  "identity-preserving local re-seal path is missing"

assert_present_in_path \
  "NO SIGNING • NO TRANSACTION • NO MAINNET" \
  "${CREDENTIAL_REKEY_ACTIVITY_PATH}" \
  "credential re-key safety statement is missing"

assert_absent_in_path \
  "signStageCDevnetProof" \
  "${CREDENTIAL_REKEY_ACTIVITY_PATH}" \
  "credential re-key activity must not invoke signing"

assert_absent_in_path \
  "createLockedDevnetVault" \
  "${CREDENTIAL_REKEY_ACTIVITY_PATH}" \
  "credential re-key activity must not create a new wallet"

echo "Checking Vercel trust boundary..."

assert_passphrase_boundary

echo "Checking Devnet lock..."

if git grep \
  --line-number \
  --fixed-strings \
  "https://api.devnet.solana.com" \
  -- crates/wallet-engine/src/lib.rs \
  >/dev/null 2>&1; then
  :
else
  fail "pinned Devnet RPC endpoint is missing"
fi

echo "Scout Wallet Lab security tripwires passed."
