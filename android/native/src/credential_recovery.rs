#[path = "credential_rekey.rs"]
mod credential_rekey;

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
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_verifyLockedDevnetPassphrase(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    vault_json: JString<'_>,
    passphrase_bytes: JByteArray<'_>,
) -> jstring {
    let vault_json: String = match env.get_string(&vault_json) {
        Ok(value) => value.into(),
        Err(_) => return java_string(env, "verification-invalid-vault-json"),
    };

    if vault_json.trim().is_empty() {
        return java_string(env, "verification-empty-vault-json");
    }

    let passphrase_bytes = match env.convert_byte_array(&passphrase_bytes) {
        Ok(value) => Zeroizing::new(value),
        Err(_) => return java_string(env, "verification-invalid-passphrase"),
    };

    if passphrase_bytes.is_empty() {
        return java_string(env, "verification-empty-passphrase");
    }

    let passphrase = match String::from_utf8(passphrase_bytes.to_vec()) {
        Ok(value) => value,
        Err(_) => return java_string(env, "verification-passphrase-not-utf8"),
    };

    let vault = match LockedVault::from_json(&vault_json) {
        Ok(vault) => vault,
        Err(_) => return java_string(env, "verification-vault-invalid"),
    };

    let account = match vault.devnet_account() {
        Ok(account) => account,
        Err(_) => return java_string(env, "verification-identity-unavailable"),
    };

    match vault.unlock(&SecretPassphrase::new(passphrase)) {
        Ok(_) => java_string(env, &format!("ok:{}", account.address())),
        Err(VaultError::DecryptionFailed) => java_string(env, "wrong-passphrase"),
        Err(_) => java_string(env, "verification-failed"),
    }
}
