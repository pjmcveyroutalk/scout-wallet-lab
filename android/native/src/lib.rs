#![deny(unsafe_code)]

use jni::{
    objects::{JByteArray, JClass, JString},
    sys::jstring,
    JNIEnv,
};
use tokio::runtime::Builder;
use wallet_engine::{
    engine_name, Cluster, DevnetRpc, LockedVault, SecretPassphrase,
};
use zeroize::Zeroizing;

fn java_string(env: JNIEnv<'_>, value: &str) -> jstring {
    match env.new_string(value) {
        Ok(output) => output.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
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
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_createLockedDevnetVault(
    env: JNIEnv<'_>,
    _class: JClass<'_>,
    passphrase_bytes: JByteArray<'_>,
) -> jstring {
    let passphrase_bytes =
        match env.convert_byte_array(&passphrase_bytes) {
            Ok(value) => Zeroizing::new(value),
            Err(_) => return java_string(env, "invalid-passphrase"),
        };

    if passphrase_bytes.is_empty() {
        return java_string(env, "empty-passphrase");
    }

    let passphrase =
        match String::from_utf8(passphrase_bytes.to_vec()) {
            Ok(value) => value,
            Err(_) => return java_string(env, "passphrase-not-utf8"),
        };

    let secret_passphrase = SecretPassphrase::new(passphrase);

    let vault = match LockedVault::generate(&secret_passphrase) {
        Ok(vault) => vault,
        Err(error) => return java_string(env, &format!("vault-generation-failed:{error}")),
    };

    let account = match vault.devnet_account() {
        Ok(account) => account,
        Err(error) => return java_string(env, &format!("address-derivation-failed:{error}")),
    };

    let vault_json = match vault.to_json() {
        Ok(encoded) => encoded,
        Err(error) => return java_string(env, &format!("vault-serialization-failed:{error}")),
    };

    let result = format!(
        "ok:{}:{}",
        account.address(),
        vault_json,
    );

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
        Err(error) => return java_string(env, &format!("vault-parse-failed:{error}")),
    };

    let account = match vault.devnet_account() {
        Ok(account) => account,
        Err(error) => return java_string(env, &format!("address-derivation-failed:{error}")),
    };

    java_string(env, &format!("ok:{}", account.address()))
}
