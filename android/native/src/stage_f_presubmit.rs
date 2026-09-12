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
        stage_f_presubmit::{
            discard_prepared_candidate, prepare_fixed_devnet_candidate, StageFCandidateToken,
        },
    },
    SecretPassphrase,
};
use zeroize::Zeroizing;

const STAGE_F_TOKEN_BYTES: usize = 16;
const STAGE_F_TOKEN_HEX_LEN: usize = STAGE_F_TOKEN_BYTES * 2;

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_prepareStageFDevnetCandidate(
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
        Err(_) => return java_string(env, "stage-f-vault-unlock-failed"),
    };

    let runtime = match Builder::new_current_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(_) => return java_string(env, "stage-f-runtime-initialization-failed"),
    };

    let prepared = match runtime.block_on(prepare_fixed_devnet_candidate(&coordinator)) {
        Ok(prepared) => prepared,
        Err(error) => return java_string(env, &format!("stage-f-presubmit-failed:{error}")),
    };

    let signature_hex = encode_hex(&prepared.signature().to_bytes());
    let token_hex = encode_hex(&prepared.candidate_token().to_bytes());
    let result = format!(
        "ok:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        prepared.public_key(),
        signature_hex,
        prepared.recent_blockhash(),
        prepared.fee_lamports(),
        prepared.balance_lamports(),
        prepared.remaining_balance_lamports(),
        prepared.simulation_slot(),
        prepared.units_consumed(),
        prepared.last_valid_block_height(),
        token_hex,
    );

    java_string(env, &result)
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_discardStageFDevnetCandidate(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    token_hex: JString<'_>,
) -> jstring {
    let token_hex: String = match env.get_string(&token_hex) {
        Ok(value) => value.into(),
        Err(_) => return java_string(env, "invalid-stage-f-token"),
    };

    let token = match decode_token(token_hex.as_str()) {
        Some(token) => token,
        None => return java_string(env, "invalid-stage-f-token"),
    };

    match discard_prepared_candidate(token) {
        Ok(()) => java_string(env, "ok"),
        Err(error) => java_string(env, &format!("stage-f-discard-failed:{error}")),
    }
}

fn decode_token(value: &str) -> Option<StageFCandidateToken> {
    if value.len() != STAGE_F_TOKEN_HEX_LEN {
        return None;
    }

    let bytes = value.as_bytes();
    let mut token = [0_u8; STAGE_F_TOKEN_BYTES];

    for (index, slot) in token.iter_mut().enumerate() {
        let high = decode_nibble(bytes[index * 2])?;
        let low = decode_nibble(bytes[index * 2 + 1])?;
        *slot = (high << 4) | low;
    }

    Some(StageFCandidateToken::from_bytes(token))
}

fn decode_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::decode_token;

    #[test]
    fn token_decoder_accepts_exact_lowercase_hex() {
        let token = decode_token("00112233445566778899aabbccddeeff");
        assert!(token.is_some());
    }

    #[test]
    fn token_decoder_rejects_wrong_length_and_non_hex() {
        assert!(decode_token("0011").is_none());
        assert!(decode_token("00112233445566778899aabbccddeezz").is_none());
        assert!(decode_token("00112233445566778899AABBCCDDEEFF").is_none());
    }
}
