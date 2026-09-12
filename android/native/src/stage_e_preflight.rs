use crate::{encode_hex, java_string};
use jni::{
    objects::{JByteArray, JClass, JString},
    sys::jstring,
    JNIEnv,
};
use tokio::runtime::Builder;
use wallet_engine::{
    recovery_words::{
        devnet_signing_coordinator::DevnetSigningCoordinator,
        stage_e_preflight::run_fixed_devnet_simulation,
    },
    SecretPassphrase,
};
use zeroize::Zeroizing;

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_simulateStageEDevnetProof(
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
        Err(_) => return java_string(env, "stage-e-vault-unlock-failed"),
    };

    let runtime = match Builder::new_current_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(_) => return java_string(env, "stage-e-runtime-initialization-failed"),
    };

    let simulated = match runtime.block_on(run_fixed_devnet_simulation(&coordinator)) {
        Ok(simulated) => simulated,
        Err(error) => return java_string(env, &format!("stage-e-preflight-failed:{error}")),
    };

    let signature_bytes = simulated.signature().to_bytes();
    let signature_hex = encode_hex(&signature_bytes);
    let result = format!(
        "ok:{}:{}:{}:{}:{}:{}",
        simulated.public_key(),
        signature_hex,
        simulated.recent_blockhash(),
        simulated.fee_lamports(),
        simulated.simulation_slot(),
        simulated.units_consumed(),
    );

    java_string(env, &result)
}
