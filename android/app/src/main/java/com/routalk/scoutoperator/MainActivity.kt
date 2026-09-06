package com.routalk.scoutoperator

import android.app.Activity
import android.os.Bundle
import android.text.InputType
import android.view.Gravity
import android.view.ViewGroup
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView

class MainActivity : Activity() {
    private data class BridgeRuntimeState(
        val identity: String,
        val status: String,
        val rpcCluster: String,
        val rpcEndpoint: String,
        val bridgeVerified: Boolean,
        val rpcVerified: Boolean,
    )

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val padding = (24 * resources.displayMetrics.density).toInt()

        fun text(value: String, size: Float): TextView =
            TextView(this).apply {
                this.text = value
                textSize = size
                setPadding(0, padding / 2, 0, padding / 2)
            }

        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            setPadding(padding, padding, padding, padding)
        }

        val bridgeRuntimeState =
            try {
                val identity = NativeBridge.engineName()
                val status = NativeBridge.bridgeStatus()
                val rpcCluster = NativeBridge.rpcCluster()
                val rpcEndpoint = NativeBridge.rpcEndpoint()

                BridgeRuntimeState(
                    identity = identity,
                    status = status,
                    rpcCluster = rpcCluster,
                    rpcEndpoint = rpcEndpoint,
                    bridgeVerified =
                        identity.isNotBlank() &&
                            identity.endsWith(":devnet") &&
                            status == "wallet-operations-locked",
                    rpcVerified =
                        rpcCluster == "devnet" &&
                            rpcEndpoint == "https://api.devnet.solana.com",
                )
            } catch (error: Throwable) {
                BridgeRuntimeState(
                    identity = "Unavailable: ${error.javaClass.simpleName}",
                    status = "bridge-load-failed",
                    rpcCluster = "Unavailable",
                    rpcEndpoint = "Unavailable",
                    bridgeVerified = false,
                    rpcVerified = false,
                )
            }

        root.addView(text("SCOUT WALLET OPERATOR", 24f))
        root.addView(text("DEVNET", 16f))

        root.addView(text("Rust bridge", 14f))
        root.addView(text(bridgeRuntimeState.identity, 16f))

        root.addView(text("Bridge status", 14f))
        root.addView(text(bridgeRuntimeState.status, 16f))

        root.addView(text("Runtime verification", 14f))
        root.addView(
            text(
                if (bridgeRuntimeState.bridgeVerified) {
                    "VERIFIED — RUST BRIDGE LOADED"
                } else {
                    "NOT VERIFIED"
                },
                16f,
            ),
        )

        root.addView(text("RPC cluster", 14f))
        root.addView(text(bridgeRuntimeState.rpcCluster, 16f))

        root.addView(text("RPC endpoint", 14f))
        root.addView(text(bridgeRuntimeState.rpcEndpoint, 16f))

        root.addView(text("RPC configuration", 14f))
        root.addView(
            text(
                if (bridgeRuntimeState.rpcVerified) {
                    "VERIFIED — DEVNET ONLY"
                } else {
                    "NOT VERIFIED"
                },
                16f,
            ),
        )

        root.addView(text("Wallet", 14f))
        root.addView(text("Not initialized", 18f))

        val createWallet = Button(this).apply {
            text = "CREATE WALLET"
            isEnabled = false
            contentDescription =
                "Create wallet unavailable until wallet activation gate is explicitly opened"
        }

        root.addView(
            createWallet,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        root.addView(text("Address", 14f))
        root.addView(text("Unavailable", 16f))

        root.addView(text("Identity", 14f))
        root.addView(text("Not verified", 16f))

        root.addView(text("Balance", 14f))
        root.addView(text("Unavailable", 16f))

        val refreshBalance = Button(this).apply {
            text = "REFRESH BALANCE"
            isEnabled = false
            inputType = InputType.TYPE_NULL
            contentDescription =
                "Balance unavailable until read-only wallet access is explicitly opened"
        }

        root.addView(
            refreshBalance,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        root.addView(text("MAINNET — DISABLED", 14f))
        root.addView(text("TRANSACTION SUBMISSION — DISABLED", 14f))

        val scrollView = ScrollView(this).apply {
            addView(
                root,
                ViewGroup.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.WRAP_CONTENT,
                ),
            )
        }

        setContentView(scrollView)
    }
}
