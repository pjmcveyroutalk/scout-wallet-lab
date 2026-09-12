use jni::{
    objects::{JByteArray, JClass, JString},
    sys::jstring,
    JNIEnv,
};
use wallet_engine::{LockedVault, SecretPassphrase, VaultError};
use zeroize::Zeroizing;

const RECOVERY_WORD_COUNT: usize = 24;
const VERIFICATION_PASSPHRASE: &str = "scout-recovery-verification-ephemeral-v1";

fn java_string(env: JNIEnv<'_>, value: &str) -> jstring {
    match env.new_string(value) {
        Ok(output) => output.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_exportLockedVaultRecoveryWords(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    vault_json: JString<'_>,
    passphrase_bytes: JByteArray<'_>,
) -> jstring {
    let vault_json: String = match env.get_string(&vault_json) {
        Ok(value) => value.into(),
        Err(_) => return java_string(env, "invalid-vault-json"),
    };

    if vault_json.trim().is_empty() {
        return java_string(env, "empty-vault-json");
    }

    let vault = match LockedVault::from_json(&vault_json) {
        Ok(vault) => vault,
        Err(_) => return java_string(env, "vault-parse-failed"),
    };

    let expected_address = match vault.devnet_account() {
        Ok(account) => account.address(),
        Err(_) => return java_string(env, "address-derivation-failed"),
    };

    let passphrase_bytes = match env.convert_byte_array(&passphrase_bytes) {
        Ok(value) => Zeroizing::new(value),
        Err(_) => return java_string(env, "invalid-passphrase"),
    };

    if passphrase_bytes.is_empty() {
        return java_string(env, "empty-passphrase");
    }

    let passphrase = match String::from_utf8(passphrase_bytes.to_vec()) {
        Ok(value) => value,
        Err(_) => return java_string(env, "passphrase-not-utf8"),
    };

    let secret_passphrase = SecretPassphrase::new(passphrase);

    let recovery_words = match vault.export_emergency_recovery_words(&secret_passphrase) {
        Ok(words) => words,
        Err(VaultError::DecryptionFailed) => return java_string(env, "wrong-passphrase"),
        Err(_) => return java_string(env, "recovery-export-failed"),
    };

    if recovery_words.public_key() != expected_address {
        return java_string(env, "recovery-identity-mismatch");
    }

    if recovery_words.words().split_whitespace().count() != RECOVERY_WORD_COUNT {
        return java_string(env, "invalid-recovery-word-count");
    }

    let result = Zeroizing::new(format!(
        "ok:{}:{}",
        expected_address,
        recovery_words.words(),
    ));

    java_string(env, result.as_str())
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_verifyLockedVaultRecoveryWords(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    vault_json: JString<'_>,
    recovery_words: JString<'_>,
) -> jstring {
    let vault_json: String = match env.get_string(&vault_json) {
        Ok(value) => value.into(),
        Err(_) => return java_string(env, "invalid-vault-json"),
    };

    if vault_json.trim().is_empty() {
        return java_string(env, "empty-vault-json");
    }

    let vault = match LockedVault::from_json(&vault_json) {
        Ok(vault) => vault,
        Err(_) => return java_string(env, "vault-parse-failed"),
    };

    let expected_address = match vault.devnet_account() {
        Ok(account) => account.address(),
        Err(_) => return java_string(env, "address-derivation-failed"),
    };

    let recovery_words: String = match env.get_string(&recovery_words) {
        Ok(value) => value.into(),
        Err(_) => return java_string(env, "invalid-recovery-words"),
    };

    let recovery_words = Zeroizing::new(recovery_words);

    if recovery_words.trim().is_empty() {
        return java_string(env, "empty-recovery-words");
    }

    if recovery_words.split_whitespace().count() != RECOVERY_WORD_COUNT {
        return java_string(env, "invalid-recovery-word-count");
    }

    let verification_passphrase =
        SecretPassphrase::new(VERIFICATION_PASSPHRASE.to_owned());

    let restored = match LockedVault::restore_from_emergency_recovery_words(
        &verification_passphrase,
        recovery_words.as_str(),
    ) {
        Ok(vault) => vault,
        Err(_) => return java_string(env, "recovery-words-invalid"),
    };

    let restored_address = match restored.devnet_account() {
        Ok(account) => account.address(),
        Err(_) => return java_string(env, "recovery-words-invalid"),
    };

    if restored_address != expected_address {
        return java_string(env, "recovery-identity-mismatch");
    }

    java_string(env, &format!("ok:{expected_address}"))
}
