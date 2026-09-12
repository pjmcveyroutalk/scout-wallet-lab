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
import android.widget.CheckBox
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast

class StageEPreflightActivity : Activity() {
    private var passphraseField: EditText? = null
    private var confirmationField: EditText? = null
    private var authorizationAcknowledgement: CheckBox? = null

    private data class PreflightPrerequisites(
        val ready: Boolean,
        val expectedAddress: String?,
        val lockedVaultJson: String?,
        val status: String,
    )

    private data class PreflightResult(
        val success: Boolean,
        val status: String,
    )

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)

        val padding = (24 * resources.displayMetrics.density).toInt()

        fun text(
            value: String,
            size: Float,
        ): TextView =
            TextView(this).apply {
                this.text = value
                textSize = size
                setPadding(0, padding / 2, 0, padding / 2)
            }

        fun fullWidth(button: Button) {
            button.layoutParams =
                ViewGroup.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.WRAP_CONTENT,
                )
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
                transformationMethod = PasswordTransformationMethod.getInstance()
                isSingleLine = true
                maxLines = 1
                isSaveEnabled = false
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

        val root =
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_HORIZONTAL
                setPadding(padding, padding, padding, padding)
            }

        root.addView(text("SCOUT STAGE E SIMULATION PREFLIGHT", 24f))
        root.addView(text("DEVNET ONLY", 18f))
        root.addView(text("SIMULATION ONLY — NO TRANSACTION SUBMISSION", 14f))
        root.addView(text("MAINNET — DISABLED", 14f))
        root.addView(text("ARBITRARY SIGNING — DISABLED", 14f))
        root.addView(
            text(
                "This gate builds and signs only the fixed Scout Stage E Devnet proof candidate, asks Devnet for its fee, and simulates it with signature verification enabled.",
                14f,
            ),
        )
        root.addView(
            text(
                "Fixed payload: scout-stage-e-devnet-simulation-proof-v1",
                13f,
            ),
        )
        root.addView(text("Maximum accepted Devnet fee: 10,000 lamports", 13f))

        val prerequisites = loadPrerequisites()

        root.addView(text("Preflight readiness", 14f))
        root.addView(text(prerequisites.status, 16f))

        root.addView(text("Verified wallet address", 14f))
        root.addView(
            text(
                prerequisites.expectedAddress ?: "Unavailable",
                16f,
            ),
        )

        val authorization =
            CheckBox(this).apply {
                text =
                    "I authorize this local Devnet signing + simulation preflight. " +
                        "No transaction will be submitted."
                isSaveEnabled = false
                isEnabled = prerequisites.ready
                contentDescription =
                    "Authorize the fixed Scout Stage E Devnet simulation preflight"
            }

        authorizationAcknowledgement = authorization
        root.addView(
            authorization,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        root.addView(text("Current wallet passphrase", 14f))
        val passphrase =
            securePassphraseField(
                hintText = "Enter current wallet passphrase",
                description = "Stage E Scout wallet passphrase",
            )
        passphraseField = passphrase
        root.addView(
            passphrase,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        root.addView(text("Confirm current passphrase", 14f))
        val confirmation =
            securePassphraseField(
                hintText = "Re-enter current passphrase",
                description = "Confirm Stage E Scout wallet passphrase",
            )
        confirmationField = confirmation
        root.addView(
            confirmation,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val preflightStatus =
            text(
                if (prerequisites.ready) {
                    "READY — EXPLICIT LOCAL AUTHORIZATION REQUIRED"
                } else {
                    "BLOCKED — PREREQUISITES NOT SATISFIED"
                },
                16f,
            ).apply {
                gravity = Gravity.CENTER
                minHeight = padding * 4
            }
        root.addView(
            preflightStatus,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val runPreflight =
            Button(this).apply {
                text = "RUN STAGE E SIMULATION PREFLIGHT"
                isEnabled = prerequisites.ready
                contentDescription =
                    "Sign and simulate the fixed Scout Stage E Devnet proof without submitting a transaction"
            }
        fullWidth(runPreflight)
        root.addView(runPreflight)

        val cancel =
            Button(this).apply {
                text = "CANCEL — DO NOT SIGN OR SIMULATE"
                contentDescription = "Cancel the Scout Stage E preflight"
            }
        fullWidth(cancel)
        root.addView(cancel)

        runPreflight.setOnClickListener {
            if (authorizationAcknowledgement?.isChecked != true) {
                preflightStatus.text =
                    "BLOCKED — EXPLICIT LOCAL STAGE E PREFLIGHT AUTHORIZATION REQUIRED"
                clearSensitiveFields()
                return@setOnClickListener
            }

            val expectedAddress = prerequisites.expectedAddress
            val lockedVaultJson = prerequisites.lockedVaultJson
            val currentPassphrase = passphraseField?.text
            val currentConfirmation = confirmationField?.text

            if (
                expectedAddress.isNullOrBlank() ||
                lockedVaultJson.isNullOrBlank() ||
                currentPassphrase == null ||
                currentConfirmation == null
            ) {
                preflightStatus.text =
                    "BLOCKED — PREFLIGHT PREREQUISITES BECAME UNAVAILABLE"
                clearSensitiveFields()
                runPreflight.isEnabled = false
                return@setOnClickListener
            }

            if (currentPassphrase.toString() != currentConfirmation.toString()) {
                preflightStatus.text =
                    "BLOCKED — CURRENT PASSPHRASE CONFIRMATION DOES NOT MATCH"
                clearSensitiveFields()
                return@setOnClickListener
            }

            when (
                val validation =
                    PassphrasePolicy.encodeCandidateForVerification(
                        currentPassphrase,
                    )
            ) {
                is PassphrasePolicy.ValidationResult.Invalid -> {
                    preflightStatus.text =
                        "BLOCKED — ${validation.reason}"
                    clearSensitiveFields()
                }

                is PassphrasePolicy.ValidationResult.Valid -> {
                    val passphraseBytes = validation.passphraseBytes
                    clearSensitiveFields()
                    runPreflight.isEnabled = false
                    cancel.isEnabled = false
                    preflightStatus.text =
                        "SIGNING FIXED CANDIDATE AND RUNNING DEVNET SIMULATION..."

                    Thread {
                        val result =
                            try {
                                runPreflight(
                                    expectedAddress = expectedAddress,
                                    lockedVaultJson = lockedVaultJson,
                                    passphraseBytes = passphraseBytes,
                                )
                            } catch (error: Throwable) {
                                PreflightResult(
                                    success = false,
                                    status =
                                        "PREFLIGHT FAILED — " +
                                            error.javaClass.simpleName,
                                )
                            } finally {
                                PassphrasePolicy.wipe(passphraseBytes)
                            }

                        runOnUiThread {
                            clearSensitiveFields()
                            authorizationAcknowledgement?.isChecked = false
                            preflightStatus.text = result.status
                            runPreflight.isEnabled = prerequisites.ready
                            cancel.isEnabled = true

                            Toast.makeText(
                                this,
                                if (result.success) {
                                    "STAGE E PREFLIGHT PASS — SIMULATED ONLY"
                                } else {
                                    "STAGE E PREFLIGHT BLOCKED / FAILED"
                                },
                                Toast.LENGTH_LONG,
                            ).show()
                        }
                    }.start()
                }
            }
        }

        cancel.setOnClickListener {
            clearSensitiveFields()
            authorizationAcknowledgement?.isChecked = false
            finish()
        }

        val scrollView =
            ScrollView(this).apply {
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
        authorizationAcknowledgement?.isChecked = false
        super.onStop()
    }

    override fun onDestroy() {
        clearSensitiveFields()
        passphraseField = null
        confirmationField = null
        authorizationAcknowledgement = null
        super.onDestroy()
    }

    private fun loadPrerequisites(): PreflightPrerequisites {
        return try {
            val identity = NativeBridge.engineName()
            val bridgeStatus = NativeBridge.bridgeStatus()
            val rpcCluster = NativeBridge.rpcCluster()
            val rpcEndpoint = NativeBridge.rpcEndpoint()

            if (
                identity.isBlank() ||
                !identity.endsWith(":devnet") ||
                bridgeStatus != "wallet-operations-locked" ||
                rpcCluster != "devnet" ||
                rpcEndpoint != "https://api.devnet.solana.com"
            ) {
                return PreflightPrerequisites(
                    ready = false,
                    expectedAddress = null,
                    lockedVaultJson = null,
                    status = "BLOCKED — DEVNET TRUST BOUNDARY NOT VERIFIED",
                )
            }

            val vaultStore = LockedVaultStore(this)
            if (!vaultStore.hasVault()) {
                return PreflightPrerequisites(
                    ready = false,
                    expectedAddress = null,
                    lockedVaultJson = null,
                    status = "BLOCKED — ENCRYPTED WALLET VAULT MISSING",
                )
            }

            val lockedVaultJson =
                vaultStore.loadVault()
                    ?: return PreflightPrerequisites(
                        ready = false,
                        expectedAddress = null,
                        lockedVaultJson = null,
                        status = "BLOCKED — ENCRYPTED WALLET VAULT UNREADABLE",
                    )

            val identityResult =
                NativeBridge.lockedVaultDevnetAddress(lockedVaultJson)

            if (!identityResult.startsWith("ok:")) {
                return PreflightPrerequisites(
                    ready = false,
                    expectedAddress = null,
                    lockedVaultJson = null,
                    status = "BLOCKED — STORED WALLET IDENTITY NOT VERIFIED",
                )
            }

            val expectedAddress = identityResult.removePrefix("ok:")
            if (expectedAddress.isBlank()) {
                return PreflightPrerequisites(
                    ready = false,
                    expectedAddress = null,
                    lockedVaultJson = null,
                    status = "BLOCKED — STORED WALLET IDENTITY IS EMPTY",
                )
            }

            PreflightPrerequisites(
                ready = true,
                expectedAddress = expectedAddress,
                lockedVaultJson = lockedVaultJson,
                status = "READY — DEVNET BRIDGE, RPC, VAULT, AND IDENTITY VERIFIED",
            )
        } catch (error: Throwable) {
            PreflightPrerequisites(
                ready = false,
                expectedAddress = null,
                lockedVaultJson = null,
                status =
                    "BLOCKED — PREREQUISITE CHECK FAILED: " +
                        error.javaClass.simpleName,
            )
        }
    }

    private fun runPreflight(
        expectedAddress: String,
        lockedVaultJson: String,
        passphraseBytes: ByteArray,
    ): PreflightResult {
        val nativeResult =
            NativeBridge.simulateStageEDevnetProof(
                lockedVaultJson,
                passphraseBytes,
            )

        if (!nativeResult.startsWith("ok:")) {
            return PreflightResult(
                success = false,
                status = "PREFLIGHT FAILED — $nativeResult",
            )
        }

        val parts = nativeResult.split(":", limit = 7)
        if (parts.size != 7 || parts[0] != "ok") {
            return PreflightResult(
                success = false,
                status = "PREFLIGHT FAILED — INVALID NATIVE RESPONSE",
            )
        }

        val returnedAddress = parts[1]
        val signatureHex = parts[2]
        val recentBlockhash = parts[3]
        val feeLamports = parts[4].toLongOrNull()
        val simulationSlot = parts[5].toLongOrNull()
        val unitsConsumed = parts[6].toLongOrNull()

        if (returnedAddress != expectedAddress) {
            return PreflightResult(
                success = false,
                status = "PREFLIGHT BLOCKED — WALLET IDENTITY MISMATCH",
            )
        }

        if (
            signatureHex.length != 128 ||
            !signatureHex.all { character ->
                character in '0'..'9' ||
                    character in 'a'..'f'
            }
        ) {
            return PreflightResult(
                success = false,
                status = "PREFLIGHT FAILED — INVALID SIGNATURE ENCODING",
            )
        }

        if (recentBlockhash.isBlank()) {
            return PreflightResult(
                success = false,
                status = "PREFLIGHT FAILED — RECENT BLOCKHASH MISSING",
            )
        }

        if (feeLamports == null || feeLamports <= 0L || feeLamports > 10_000L) {
            return PreflightResult(
                success = false,
                status = "PREFLIGHT BLOCKED — DEVNET FEE OUTSIDE FIXED BOUNDARY",
            )
        }

        if (simulationSlot == null || simulationSlot <= 0L) {
            return PreflightResult(
                success = false,
                status = "PREFLIGHT FAILED — SIMULATION SLOT INVALID",
            )
        }

        if (unitsConsumed == null || unitsConsumed <= 0L) {
            return PreflightResult(
                success = false,
                status = "PREFLIGHT FAILED — SIMULATION UNITS INVALID",
            )
        }

        return PreflightResult(
            success = true,
            status =
                buildString {
                    append("STAGE E SIMULATION PREFLIGHT — PASS")
                    append("\n\n")
                    append("Address: ")
                    append(returnedAddress)
                    append("\n\n")
                    append("Signature: ")
                    append(signatureHex)
                    append("\n\n")
                    append("Recent blockhash: ")
                    append(recentBlockhash)
                    append("\n")
                    append("Fee estimate: ")
                    append(feeLamports)
                    append(" lamports")
                    append("\n")
                    append("Simulation slot: ")
                    append(simulationSlot)
                    append("\n")
                    append("Units consumed: ")
                    append(unitsConsumed)
                    append("\n\n")
                    append("TRANSACTION SUBMISSION: DISABLED")
                    append("\n")
                    append("MAINNET: DISABLED")
                    append("\n")
                    append("ARBITRARY SIGNING: DISABLED")
                },
        )
    }

    private fun clearSensitiveFields() {
        passphraseField?.text?.clear()
        confirmationField?.text?.clear()
    }
}
