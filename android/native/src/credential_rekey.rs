use super::java_string;
use jni::{
    objects::{JByteArray, JClass, JString},
    sys::jstring,
    JNIEnv,
};
use wallet_engine::{LockedVault, SecretPassphrase, VaultError};
use zeroize::Zeroizing;

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_rekeyLockedDevnetVault(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    vault_json: JString<'_>,
    current_passphrase_bytes: JByteArray<'_>,
    new_passphrase_bytes: JByteArray<'_>,
) -> jstring {
    let vault_json: String = match env.get_string(&vault_json) {
        Ok(value) => value.into(),
        Err(_) => return java_string(env, "rekey-invalid-vault-json"),
    };

    if vault_json.trim().is_empty() {
        return java_string(env, "rekey-empty-vault-json");
    }

    let current_passphrase_bytes = match env.convert_byte_array(&current_passphrase_bytes) {
        Ok(value) => Zeroizing::new(value),
        Err(_) => return java_string(env, "rekey-invalid-current-passphrase"),
    };

    if current_passphrase_bytes.is_empty() {
        return java_string(env, "rekey-empty-current-passphrase");
    }

    let new_passphrase_bytes = match env.convert_byte_array(&new_passphrase_bytes) {
        Ok(value) => Zeroizing::new(value),
        Err(_) => return java_string(env, "rekey-invalid-new-passphrase"),
    };

    if new_passphrase_bytes.is_empty() {
        return java_string(env, "rekey-empty-new-passphrase");
    }

    let current_passphrase = match String::from_utf8(current_passphrase_bytes.to_vec()) {
        Ok(value) => SecretPassphrase::new(value),
        Err(_) => return java_string(env, "rekey-current-passphrase-not-utf8"),
    };

    let new_passphrase = match String::from_utf8(new_passphrase_bytes.to_vec()) {
        Ok(value) => SecretPassphrase::new(value),
        Err(_) => return java_string(env, "rekey-new-passphrase-not-utf8"),
    };

    let vault = match LockedVault::from_json(&vault_json) {
        Ok(vault) => vault,
        Err(_) => return java_string(env, "rekey-vault-invalid"),
    };

    let expected_account = match vault.devnet_account() {
        Ok(account) => account,
        Err(_) => return java_string(env, "rekey-identity-unavailable"),
    };

    let recovery_words = match vault.export_emergency_recovery_words(&current_passphrase) {
        Ok(words) => words,
        Err(VaultError::DecryptionFailed) => return java_string(env, "wrong-passphrase"),
        Err(_) => return java_string(env, "rekey-current-vault-unlock-failed"),
    };

    if recovery_words.public_key() != expected_account.address() {
        return java_string(env, "rekey-source-identity-mismatch");
    }

    let replacement = match LockedVault::restore_from_emergency_recovery_words(
        &new_passphrase,
        recovery_words.words(),
    ) {
        Ok(vault) => vault,
        Err(_) => return java_string(env, "rekey-reseal-failed"),
    };

    let replacement_account = match replacement.devnet_account() {
        Ok(account) => account,
        Err(_) => return java_string(env, "rekey-replacement-identity-unavailable"),
    };

    if replacement_account.address() != expected_account.address() {
        return java_string(env, "rekey-replacement-identity-mismatch");
    }

    match replacement.unlock(&new_passphrase) {
        Ok(unlocked) if unlocked.public_key() == expected_account.address().to_bytes() => {}
        Ok(_) => return java_string(env, "rekey-replacement-unlock-identity-mismatch"),
        Err(_) => return java_string(env, "rekey-replacement-unlock-failed"),
    }

    let replacement_json = match replacement.to_json() {
        Ok(encoded) => encoded,
        Err(_) => return java_string(env, "rekey-replacement-serialization-failed"),
    };

    java_string(
        env,
        &format!(
            "ok:{}:{}",
            replacement_account.address(),
            replacement_json,
        ),
    )
}
