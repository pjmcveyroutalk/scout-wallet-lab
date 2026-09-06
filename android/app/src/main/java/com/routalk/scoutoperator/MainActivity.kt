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

    private data class WalletCreationResult(
        val success: Boolean,
        val address: String?,
        val status: String,
        val retryAllowed: Boolean,
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
                            "VERIFIED — DEVNET LIVE\n" +
                                "Block height: $blockHeight"
                    } else {
                        networkStatus.text =
                            "DEVNET CHECK FAILED\n$result"
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

        val storageStatus =
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
            )

        root.addView(storageStatus)

        root.addView(text("Wallet", 14f))

        val walletStatus =
            text(
                if (vaultIdentityState.verified) {
                    "Locked encrypted vault detected"
                } else {
                    "Not initialized"
                },
                18f,
            )

        root.addView(walletStatus)

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

        val creationStatus =
            text(
                when {
                    vaultStorageState.hasStoredEntry ->
                        "Wallet creation locked — vault storage already occupied."

                    !bridgeRuntimeState.bridgeVerified ->
                        "Wallet creation locked — Rust bridge not verified."

                    !bridgeRuntimeState.rpcVerified ->
                        "Wallet creation locked — Devnet RPC configuration not verified."

                    else ->
                        "READY — encrypted Devnet wallet creation gate open."
                },
                14f,
            )

        root.addView(creationStatus)

        val createWallet = Button(this).apply {
            text = "CREATE WALLET"

            isEnabled =
                bridgeRuntimeState.bridgeVerified &&
                    bridgeRuntimeState.rpcVerified &&
                    !vaultStorageState.hasStoredEntry

            contentDescription =
                "Create encrypted Scout Devnet wallet"
        }

        root.addView(
            createWallet,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        root.addView(text("Address", 14f))

        val addressStatus =
            text(
                vaultIdentityState.address ?: "Unavailable",
                16f,
            )

        root.addView(addressStatus)

        root.addView(text("Identity", 14f))

        val identityStatus =
            text(
                vaultIdentityState.status,
                16f,
            )

        root.addView(identityStatus)

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

        if (vaultIdentityState.verified) {
            passphrase.isEnabled = false
            confirmation.isEnabled = false
            createWallet.isEnabled = false
        }

        createWallet.setOnClickListener {
            val currentPassphrase =
                passphraseField?.text

            val currentConfirmation =
                confirmationField?.text

            if (
                currentPassphrase == null ||
                currentConfirmation == null
            ) {
                creationStatus.text =
                    "WALLET CREATION BLOCKED — PASSPHRASE INPUT UNAVAILABLE"

                clearSensitiveFields()
                return@setOnClickListener
            }

            when (
                val validation =
                    PassphrasePolicy.validateAndEncode(
                        currentPassphrase,
                        currentConfirmation,
                    )
            ) {
                is PassphrasePolicy.ValidationResult.Invalid -> {
                    creationStatus.text =
                        "WALLET CREATION BLOCKED — ${validation.reason}"

                    clearSensitiveFields()
                }

                is PassphrasePolicy.ValidationResult.Valid -> {
                    val passphraseBytes =
                        validation.passphraseBytes

                    clearSensitiveFields()

                    createWallet.isEnabled = false
                    passphrase.isEnabled = false
                    confirmation.isEnabled = false

                    creationStatus.text =
                        "CREATING ENCRYPTED DEVNET VAULT..."

                    Thread {
                        val creationResult =
                            try {
                                createAndVerifyWallet(
                                    passphraseBytes,
                                )
                            } catch (error: Throwable) {
                                val storedEntryPresent =
                                    try {
                                        LockedVaultStore(this).hasVault()
                                    } catch (_: Throwable) {
                                        true
                                    }

                                WalletCreationResult(
                                    success = false,
                                    address = null,
                                    status =
                                        "WALLET CREATION FAILED — " +
                                            error.javaClass.simpleName,
                                    retryAllowed =
                                        !storedEntryPresent,
                                )
                            } finally {
                                PassphrasePolicy.wipe(
                                    passphraseBytes,
                                )
                            }

                        runOnUiThread {
                            clearSensitiveFields()

                            if (creationResult.success) {
                                storageStatus.text =
                                    "VERIFIED — ENCRYPTED VAULT STORED"

                                walletStatus.text =
                                    "Locked encrypted vault detected"

                                addressStatus.text =
                                    creationResult.address ?: "Unavailable"

                                identityStatus.text =
                                    "VERIFIED — DEVNET PUBLIC IDENTITY"

                                creationStatus.text =
                                    creationResult.status

                                createWallet.isEnabled = false
                                passphrase.isEnabled = false
                                confirmation.isEnabled = false
                            } else {
                                creationStatus.text =
                                    creationResult.status

                                createWallet.isEnabled =
                                    creationResult.retryAllowed &&
                                        bridgeRuntimeState.bridgeVerified &&
                                        bridgeRuntimeState.rpcVerified

                                passphrase.isEnabled =
                                    creationResult.retryAllowed

                                confirmation.isEnabled =
                                    creationResult.retryAllowed
                            }
                        }
                    }.start()
                }
            }
        }
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

    private fun createAndVerifyWallet(
        passphraseBytes: ByteArray,
    ): WalletCreationResult {
        val vaultStore =
            LockedVaultStore(this)

        if (vaultStore.hasVault()) {
            return WalletCreationResult(
                success = false,
                address = null,
                status =
                    "WALLET CREATION BLOCKED — VAULT STORAGE ALREADY OCCUPIED",
                retryAllowed = false,
            )
        }

        val nativeResult =
            NativeBridge.createLockedDevnetVault(
                passphraseBytes,
            )

        if (!nativeResult.startsWith("ok:")) {
            return WalletCreationResult(
                success = false,
                address = null,
                status =
                    "WALLET CREATION FAILED — $nativeResult",
                retryAllowed = true,
            )
        }

        val resultParts =
            nativeResult.split(
                ":",
                limit = 3,
            )

        if (
            resultParts.size != 3 ||
            resultParts[0] != "ok"
        ) {
            return WalletCreationResult(
                success = false,
                address = null,
                status =
                    "WALLET CREATION FAILED — INVALID NATIVE RESPONSE",
                retryAllowed = true,
            )
        }

        val generatedAddress =
            resultParts[1]

        val lockedVaultJson =
            resultParts[2]

        if (
            generatedAddress.isBlank() ||
            lockedVaultJson.isBlank()
        ) {
            return WalletCreationResult(
                success = false,
                address = null,
                status =
                    "WALLET CREATION FAILED — INCOMPLETE NATIVE RESPONSE",
                retryAllowed = true,
            )
        }

        if (!vaultStore.saveVault(lockedVaultJson)) {
            return WalletCreationResult(
                success = false,
                address = null,
                status =
                    "WALLET CREATION FAILED — ENCRYPTED VAULT NOT STORED",
                retryAllowed = true,
            )
        }

        val storedVaultJson =
            vaultStore.loadVault()

        if (storedVaultJson == null) {
            return failureAfterStoredVault(
                vaultStore = vaultStore,
                status =
                    "WALLET CREATION FAILED — STORED VAULT NOT READABLE",
            )
        }

        val verificationResult =
            NativeBridge.lockedVaultDevnetAddress(
                storedVaultJson,
            )

        if (!verificationResult.startsWith("ok:")) {
            return failureAfterStoredVault(
                vaultStore = vaultStore,
                status =
                    "WALLET CREATION FAILED — STORED VAULT IDENTITY NOT VERIFIED",
            )
        }

        val verifiedAddress =
            verificationResult.removePrefix("ok:")

        if (
            verifiedAddress.isBlank() ||
            verifiedAddress != generatedAddress
        ) {
            return failureAfterStoredVault(
                vaultStore = vaultStore,
                status =
                    "WALLET CREATION FAILED — ADDRESS VERIFICATION MISMATCH",
            )
        }

        return WalletCreationResult(
            success = true,
            address = verifiedAddress,
            status =
                "VERIFIED — ENCRYPTED DEVNET WALLET CREATED AND RELOADED",
            retryAllowed = false,
        )
    }

    private fun failureAfterStoredVault(
        vaultStore: LockedVaultStore,
        status: String,
    ): WalletCreationResult {
        val cleared =
            try {
                vaultStore.clearVault()
            } catch (_: Throwable) {
                false
            }

        return if (cleared) {
            WalletCreationResult(
                success = false,
                address = null,
                status = status,
                retryAllowed = true,
            )
        } else {
            WalletCreationResult(
                success = false,
                address = null,
                status =
                    "$status — VAULT CLEANUP FAILED",
                retryAllowed = false,
            )
        }
    }

    private fun clearSensitiveFields() {
        passphraseField?.text?.clear()
        confirmationField?.text?.clear()
    }
}
