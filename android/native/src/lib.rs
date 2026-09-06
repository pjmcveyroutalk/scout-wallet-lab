#![deny(unsafe_code)]

use jni::{objects::JClass, sys::jstring, JNIEnv};
use wallet_engine::{engine_name, Cluster};

fn java_string(mut env: JNIEnv<'_>, value: &str) -> jstring {
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
