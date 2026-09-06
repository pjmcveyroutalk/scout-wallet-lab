#![deny(unsafe_code)]

use jni::{objects::JClass, sys::jstring, JNIEnv};
use tokio::runtime::Builder;
use wallet_engine::{engine_name, Cluster, DevnetRpc};

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
