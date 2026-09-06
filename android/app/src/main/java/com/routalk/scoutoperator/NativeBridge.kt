package com.routalk.scoutoperator

internal object NativeBridge {
    private const val LIBRARY_NAME = "scout_operator_native"

    init {
        System.loadLibrary(LIBRARY_NAME)
    }

    external fun engineName(): String

    external fun bridgeStatus(): String
}
