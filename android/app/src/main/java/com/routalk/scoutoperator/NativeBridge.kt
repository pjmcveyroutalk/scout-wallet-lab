package com.routalk.scoutoperator

internal object NativeBridge {
    private const val LIBRARY_NAME = "scout_operator_native"

    init {
        System.loadLibrary(LIBRARY_NAME)
    }

    external fun engineName(): String

    external fun bridgeStatus(): String

    external fun rpcCluster(): String

    external fun rpcEndpoint(): String

    external fun devnetBlockHeight(): String

    external fun createLockedDevnetVault(passphrase: String): String
}
