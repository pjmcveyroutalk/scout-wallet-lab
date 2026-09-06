package com.routalk.scoutoperator

import android.app.Activity
import android.os.Bundle
import android.text.InputType
import android.view.Gravity
import android.view.ViewGroup
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView

class MainActivity : Activity() {
    private data class BridgeRuntimeState(
        val identity: String,
        val status: String,
        val verified: Boolean,
    )

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val padding = (24 * resources.displayMetrics.density).toInt()

        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            setPadding(padding, padding, padding, padding)
        }

        fun text(value: String, size: Float): TextView =
            TextView(this).apply {
                this.text = value
                textSize = size
                setPadding(0, padding / 2, 0, padding / 2)
            }

        root.addView(text("SCOUT WALLET OPERATOR", 24f))
        root.addView(text("DEVNET", 16f))

        root.addView(text("Rust bridge", 14f))

        val bridgeRuntimeState =
            try {
                val identity = NativeBridge.engineName()
                val status = NativeBridge.bridgeStatus()

                BridgeRuntimeState(
                    identity = identity,
                    status = status,
                    verified =
                        identity.isNotBlank() &&
                            identity.endsWith(":devnet") &&
                            status == "wallet-operations-locked",
                )
            } catch (error: Throwable) {
                BridgeRuntimeState(
                    identity = "Unavailable: ${error.javaClass.simpleName}",
                    status = "bridge-load-failed",
                    verified = false,
                )
            }

        root.addView(text(bridgeRuntimeState.identity, 16f))

        root.addView(text("Bridge status", 14f))
        root.addView(text(bridgeRuntimeState.status, 16f))

        root.addView(text("Runtime verification", 14f))
        root.addView(
            text(
                if (bridgeRuntimeState.verified) {
                    "VERIFIED — RUST BRIDGE LOADED"
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

        setContentView(root)
    }
}
