#![deny(unsafe_code)]

use jni::{objects::JClass, sys::jstring, JNIEnv};
use wallet_engine::engine_name;

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_engineName(
    env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jstring {
    let value = format!("{}:devnet", engine_name());

    match env.new_string(value) {
        Ok(output) => output.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[allow(unsafe_code)]
#[no_mangle]
pub extern "system" fn Java_com_routalk_scoutoperator_NativeBridge_bridgeStatus(
    env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jstring {
    let value = "wallet-operations-locked";

    match env.new_string(value) {
        Ok(output) => output.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
