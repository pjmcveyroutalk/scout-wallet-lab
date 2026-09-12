#![deny(unsafe_code)]

mod account_history;
mod credential_recovery;
mod recovery_backup;
mod stage_e_preflight;
mod stage_f_presubmit;

use account_history::fetch_locked_vault_devnet_history;
use jni::{
    objects::{JByteArray, JClass, JString},
    sys::jstring,
    JNIEnv,
};
use recovery_backup::{create_locked_vault_backup, validate_locked_vault_backup};
use tokio::runtime::Builder;
use wallet_engine::{
    engine_name, recovery_words::devnet_signing_coordinator::DevnetSigningCoordinator, Cluster,
    DevnetRpc, LockedVault, SecretPassphrase,
};
use zeroize::Zeroizing;

fn java_string(env: JNIEnv<'_>, value: &str) -> jstring {
    match env.new_string(value) {
        Ok(output) => output.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let value = usize::from(*byte);
        encoded.push(char::from(HEX[value >> 4]));
        encoded.push(char::from(HEX[value & 0x0f]));
    }
    encoded
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_engineName(
    env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jstring {
    let value = format!("{}:{}", engine_name(), Cluster::Devnet.rpc_name());
    java_string(env, &value)
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_bridgeStatus(
    env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jstring {
    java_string(env, "wallet-operations-locked")
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_rpcCluster(
    env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jstring {
    java_string(env, Cluster::Devnet.rpc_name())
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_rpcEndpoint(
    env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jstring {
    java_string(env, Cluster::Devnet.rpc_url())
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_devnetBlockHeight(
    env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jstring {
    let runtime = match Builder::new_current_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(_) => return java_string(env, "runtime-initialization-failed"),
    };

    let rpc = match DevnetRpc::new() {
        Ok(rpc) => rpc,
        Err(error) => return java_string(env, &format!("rpc-init-failed:{error}")),
    };

    match runtime.block_on(rpc.get_block_height()) {
        Ok(block_height) => java_string(env, &format!("ok:{block_height}")),
        Err(error) => java_string(env, &format!("rpc-failed:{error}")),
    }
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_signStageCDevnetProof(
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

    let coordinator = match DevnetSigningCoordinator::unlock_vault_json(
        &vault_json,
        SecretPassphrase::new(passphrase),
    ) {
        Ok(coordinator) => coordinator,
        Err(_) => return java_string(env, "stage-c-vault-unlock-failed"),
    };

    let runtime = match Builder::new_current_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(_) => return java_string(env, "runtime-initialization-failed"),
    };

    let rpc = match DevnetRpc::new() {
        Ok(rpc) => rpc,
        Err(_) => return java_string(env, "stage-c-rpc-init-failed"),
    };

    let lease = match runtime.block_on(rpc.resolve_fresh_blockhash()) {
        Ok(lease) => lease,
        Err(_) => return java_string(env, "stage-c-blockhash-fetch-failed"),
    };

    let signed = match coordinator.sign_stage_c_proof(lease) {
        Ok(signed) => signed,
        Err(_) => return java_string(env, "stage-c-signing-failed"),
    };

    let signature_bytes = signed.signature().to_bytes();
    let signature_hex = encode_hex(&signature_bytes);
    let result = format!(
        "ok:{}:{}:{}:{}",
        signed.public_key(),
        signature_hex,
        signed.recent_blockhash(),
        signed.reserved_lamports(),
    );

    java_string(env, &result)
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_createLockedDevnetVault(
    env: JNIEnv<'_>,
    _class: JClass<'_>,
    passphrase_bytes: JByteArray<'_>,
) -> jstring {
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

    let vault = match LockedVault::generate(&secret_passphrase) {
        Ok(vault) => vault,
        Err(error) => {
            return java_string(env, &format!("vault-generation-failed:{error}"));
        }
    };

    let account = match vault.devnet_account() {
        Ok(account) => account,
        Err(error) => {
            return java_string(env, &format!("address-derivation-failed:{error}"));
        }
    };

    let vault_json = match vault.to_json() {
        Ok(encoded) => encoded,
        Err(error) => {
            return java_string(env, &format!("vault-serialization-failed:{error}"));
        }
    };

    let result = format!("ok:{}:{}", account.address(), vault_json,);

    java_string(env, &result)
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_lockedVaultDevnetAddress(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    vault_json: JString<'_>,
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
        Err(error) => {
            return java_string(env, &format!("vault-parse-failed:{error}"));
        }
    };

    let account = match vault.devnet_account() {
        Ok(account) => account,
        Err(error) => {
            return java_string(env, &format!("address-derivation-failed:{error}"));
        }
    };

    java_string(env, &format!("ok:{}", account.address()))
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_lockedVaultDevnetBalance(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    vault_json: JString<'_>,
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
        Err(error) => {
            return java_string(env, &format!("vault-parse-failed:{error}"));
        }
    };

    let account = match vault.devnet_account() {
        Ok(account) => account,
        Err(error) => {
            return java_string(env, &format!("address-derivation-failed:{error}"));
        }
    };

    let runtime = match Builder::new_current_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(_) => return java_string(env, "runtime-initialization-failed"),
    };

    let rpc = match DevnetRpc::new() {
        Ok(rpc) => rpc,
        Err(error) => return java_string(env, &format!("rpc-init-failed:{error}")),
    };

    match runtime.block_on(rpc.get_balance(account.address())) {
        Ok(lamports) => java_string(env, &format!("ok:{}:{lamports}", account.address(),)),
        Err(error) => java_string(env, &format!("rpc-failed:{error}")),
    }
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_lockedVaultDevnetHistory(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    vault_json: JString<'_>,
) -> jstring {
    let vault_json: String = match env.get_string(&vault_json) {
        Ok(value) => value.into(),
        Err(_) => return java_string(env, "invalid-vault-json"),
    };

    if vault_json.trim().is_empty() {
        return java_string(env, "empty-vault-json");
    }

    let runtime = match Builder::new_current_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(_) => return java_string(env, "runtime-initialization-failed"),
    };

    match runtime.block_on(fetch_locked_vault_devnet_history(&vault_json)) {
        Ok(records) => {
            let history = records
                .iter()
                .map(|record| format!("{}:{}", record.signature(), record.slot(),))
                .collect::<Vec<_>>()
                .join(",");

            java_string(env, &format!("ok:{history}"))
        }
        Err(error) => java_string(env, &format!("history-failed:{error}")),
    }
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_createLockedVaultBackup(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    vault_json: JString<'_>,
) -> jstring {
    let vault_json: String = match env.get_string(&vault_json) {
        Ok(value) => value.into(),
        Err(_) => return java_string(env, "invalid-vault-json"),
    };

    match create_locked_vault_backup(&vault_json) {
        Ok(backup_json) => java_string(env, &format!("ok:{backup_json}")),
        Err(error) => java_string(env, &format!("backup-failed:{error}")),
    }
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_validateLockedVaultBackup(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    backup_json: JString<'_>,
) -> jstring {
    let backup_json: String = match env.get_string(&backup_json) {
        Ok(value) => value.into(),
        Err(_) => return java_string(env, "invalid-backup-json"),
    };

    match validate_locked_vault_backup(&backup_json) {
        Ok(validated) => java_string(env, &format!("ok:{}", validated.public_address(),)),
        Err(error) => java_string(env, &format!("backup-validation-failed:{error}")),
    }
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_extractValidatedLockedVaultBackup(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    backup_json: JString<'_>,
) -> jstring {
    let backup_json: String = match env.get_string(&backup_json) {
        Ok(value) => value.into(),
        Err(_) => return java_string(env, "invalid-backup-json"),
    };

    match validate_locked_vault_backup(&backup_json) {
        Ok(validated) => java_string(
            env,
            &format!(
                "ok:{}:{}",
                validated.public_address(),
                validated.locked_vault_json(),
            ),
        ),
        Err(error) => java_string(env, &format!("backup-extraction-failed:{error}")),
    }
}
