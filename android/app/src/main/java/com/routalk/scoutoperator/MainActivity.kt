package com.routalk.scoutoperator

import android.app.Activity
import android.os.Build
import android.os.Bundle
import android.text.InputType
import android.text.method.PasswordTransformationMethod
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.view.inputmethod.EditorInfo
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView

class MainActivity : Activity() {
    private var passphraseField: EditText? = null
    private var confirmationField: EditText? = null

    private data class BridgeRuntimeState(
        val identity: String,
        val status: String,
        val rpcCluster: String,
        val rpcEndpoint: String,
        val bridgeVerified: Boolean,
        val rpcVerified: Boolean,
    )

    private data class VaultStorageState(
        val hasStoredEntry: Boolean,
        val lockedVaultJson: String?,
    ) {
        val hasReadableVault: Boolean
            get() = !lockedVaultJson.isNullOrBlank()
    }

    private data class VaultIdentityState(
        val verified: Boolean,
        val address: String?,
        val status: String,
    )

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)

        val padding = (24 * resources.displayMetrics.density).toInt()

        fun text(value: String, size: Float): TextView =
            TextView(this).apply {
                this.text = value
                textSize = size
                setPadding(0, padding / 2, 0, padding / 2)
            }

        fun securePassphraseField(
            hintText: String,
            description: String,
        ): EditText =
            EditText(this).apply {
                hint = hintText
                inputType =
                    InputType.TYPE_CLASS_TEXT or
                        InputType.TYPE_TEXT_VARIATION_PASSWORD or
                        InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS

                transformationMethod =
                    PasswordTransformationMethod.getInstance()

                isSingleLine = true
                maxLines = 1

                contentDescription = description

                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    importantForAutofill =
                        View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS

                    setAutofillHints(null)

                    imeOptions =
                        imeOptions or
                            EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING
                }
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

        val vaultStorageState =
            try {
                val vaultStore = LockedVaultStore(this)

                VaultStorageState(
                    hasStoredEntry = vaultStore.hasVault(),
                    lockedVaultJson = vaultStore.loadVault(),
                )
            } catch (_: Throwable) {
                VaultStorageState(
                    hasStoredEntry = false,
                    lockedVaultJson = null,
                )
            }

        val vaultIdentityState =
            if (
                bridgeRuntimeState.bridgeVerified &&
                vaultStorageState.hasReadableVault
            ) {
                try {
                    val result =
                        NativeBridge.lockedVaultDevnetAddress(
                            vaultStorageState.lockedVaultJson.orEmpty(),
                        )

                    if (result.startsWith("ok:")) {
                        val address = result.removePrefix("ok:")

                        if (address.isNotBlank()) {
                            VaultIdentityState(
                                verified = true,
                                address = address,
                                status = "VERIFIED — DEVNET PUBLIC IDENTITY",
                            )
                        } else {
                            VaultIdentityState(
                                verified = false,
                                address = null,
                                status = "NOT VERIFIED — EMPTY ADDRESS",
                            )
                        }
                    } else {
                        VaultIdentityState(
                            verified = false,
                            address = null,
                            status = "NOT VERIFIED — $result",
                        )
                    }
                } catch (error: Throwable) {
                    VaultIdentityState(
                        verified = false,
                        address = null,
                        status =
                            "NOT VERIFIED — bridge-call-failed:" +
                                error.javaClass.simpleName,
                    )
                }
            } else {
                VaultIdentityState(
                    verified = false,
                    address = null,
                    status =
                        if (vaultStorageState.hasReadableVault) {
                            "NOT VERIFIED — RUST BRIDGE UNAVAILABLE"
                        } else {
                            "NOT VERIFIED — NO VAULT"
                        },
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

        root.addView(text("Devnet network", 14f))

        val networkStatus = text("NOT CHECKED", 16f)
        root.addView(networkStatus)

        val checkDevnet = Button(this).apply {
            text = "CHECK DEVNET"
            isEnabled =
                bridgeRuntimeState.bridgeVerified &&
                    bridgeRuntimeState.rpcVerified

            contentDescription =
                "Perform read-only Solana Devnet network health check"
        }

        checkDevnet.setOnClickListener {
            checkDevnet.isEnabled = false
            networkStatus.text = "CHECKING DEVNET..."

            Thread {
                val result =
                    try {
                        NativeBridge.devnetBlockHeight()
                    } catch (error: Throwable) {
                        "bridge-call-failed:${error.javaClass.simpleName}"
                    }

                runOnUiThread {
                    if (result.startsWith("ok:")) {
                        val blockHeight = result.removePrefix("ok:")
                        networkStatus.text =
                            "VERIFIED — DEVNET LIVE\nBlock height: $blockHeight"
                    } else {
                        networkStatus.text = "DEVNET CHECK FAILED\n$result"
                    }

                    checkDevnet.isEnabled =
                        bridgeRuntimeState.bridgeVerified &&
                            bridgeRuntimeState.rpcVerified
                }
            }.start()
        }

        root.addView(
            checkDevnet,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        root.addView(text("Wallet storage", 14f))
        root.addView(
            text(
                when {
                    vaultStorageState.hasReadableVault ->
                        "VERIFIED — ENCRYPTED VAULT STORED"

                    vaultStorageState.hasStoredEntry ->
                        "STORAGE ENTRY PRESENT — VAULT UNREADABLE"

                    else ->
                        "NO ENCRYPTED VAULT STORED"
                },
                16f,
            ),
        )

        root.addView(text("Wallet", 14f))
        root.addView(
            text(
                if (vaultIdentityState.verified) {
                    "Locked encrypted vault detected"
                } else {
                    "Not initialized"
                },
                18f,
            ),
        )

        root.addView(text("Wallet passphrase", 14f))

        val passphrase =
            securePassphraseField(
                hintText = "16–128 printable ASCII characters",
                description = "Scout wallet passphrase",
            )

        passphraseField = passphrase

        root.addView(
            passphrase,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        root.addView(text("Confirm passphrase", 14f))

        val confirmation =
            securePassphraseField(
                hintText = "Re-enter passphrase",
                description = "Confirm Scout wallet passphrase",
            )

        confirmationField = confirmation

        root.addView(
            confirmation,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        root.addView(
            text(
                "Passphrase entry staged — wallet creation remains locked.",
                14f,
            ),
        )

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
        root.addView(
            text(
                vaultIdentityState.address ?: "Unavailable",
                16f,
            ),
        )

        root.addView(text("Identity", 14f))
        root.addView(text(vaultIdentityState.status, 16f))

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

    override fun onStop() {
        clearSensitiveFields()
        super.onStop()
    }

    override fun onDestroy() {
        clearSensitiveFields()
        passphraseField = null
        confirmationField = null
        super.onDestroy()
    }

    private fun clearSensitiveFields() {
        passphraseField?.text?.clear()
        confirmationField?.text?.clear()
    }
}
