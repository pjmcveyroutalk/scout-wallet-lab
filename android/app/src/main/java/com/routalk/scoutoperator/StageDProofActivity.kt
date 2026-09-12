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

class StageDProofActivity : Activity() {
    private var passphraseField: EditText? = null
    private var confirmationField: EditText? = null
    private var authorizationAcknowledgement: CheckBox? = null

    private data class ProofPrerequisites(
        val ready: Boolean,
        val expectedAddress: String?,
        val lockedVaultJson: String?,
        val status: String,
    )

    private data class ProofResult(
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

        root.addView(text("SCOUT STAGE D PHYSICAL PROOF", 24f))
        root.addView(text("DEVNET ONLY", 18f))
        root.addView(text("TRANSACTION SUBMISSION — DISABLED", 14f))
        root.addView(text("MAINNET — DISABLED", 14f))
        root.addView(text("ARBITRARY SIGNING — DISABLED", 14f))
        root.addView(
            text(
                "This gate signs only the fixed Scout Devnet proof operation and returns public verification metadata.",
                14f,
            ),
        )
        root.addView(
            text(
                "Fixed payload: scout-stage-c-devnet-signing-proof-v1",
                13f,
            ),
        )
        root.addView(text("Reserved exposure: 1 lamport", 13f))

        val prerequisites = loadPrerequisites()

        root.addView(text("Proof readiness", 14f))
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
                    "I authorize this local physical Devnet signature proof. " +
                        "No transaction will be submitted."
                isSaveEnabled = false
                isEnabled = prerequisites.ready
                contentDescription =
                    "Authorize the fixed local Scout Stage D Devnet signature proof"
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
                description = "Stage D Scout wallet passphrase",
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
                description = "Confirm Stage D Scout wallet passphrase",
            )
        confirmationField = confirmation
        root.addView(
            confirmation,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val proofStatus =
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
            proofStatus,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val generateProof =
            Button(this).apply {
                text = "GENERATE LOCAL DEVNET SIGNATURE PROOF"
                isEnabled = prerequisites.ready
                contentDescription =
                    "Generate the fixed Scout Stage D Devnet signature proof without submitting a transaction"
            }
        fullWidth(generateProof)
        root.addView(generateProof)

        val cancel =
            Button(this).apply {
                text = "CANCEL — DO NOT SIGN"
                contentDescription = "Cancel the Scout Stage D proof without signing"
            }
        fullWidth(cancel)
        root.addView(cancel)

        generateProof.setOnClickListener {
            if (authorizationAcknowledgement?.isChecked != true) {
                proofStatus.text =
                    "BLOCKED — EXPLICIT LOCAL STAGE D AUTHORIZATION REQUIRED"
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
                proofStatus.text =
                    "BLOCKED — PROOF PREREQUISITES BECAME UNAVAILABLE"
                clearSensitiveFields()
                generateProof.isEnabled = false
                return@setOnClickListener
            }

            if (currentPassphrase.toString() != currentConfirmation.toString()) {
                proofStatus.text =
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
                    proofStatus.text =
                        "BLOCKED — ${validation.reason}"
                    clearSensitiveFields()
                }

                is PassphrasePolicy.ValidationResult.Valid -> {
                    val passphraseBytes = validation.passphraseBytes
                    clearSensitiveFields()
                    generateProof.isEnabled = false
                    cancel.isEnabled = false
                    proofStatus.text =
                        "GENERATING FIXED LOCAL DEVNET SIGNATURE PROOF..."

                    Thread {
                        val result =
                            try {
                                runProof(
                                    expectedAddress = expectedAddress,
                                    lockedVaultJson = lockedVaultJson,
                                    passphraseBytes = passphraseBytes,
                                )
                            } catch (error: Throwable) {
                                ProofResult(
                                    success = false,
                                    status =
                                        "PROOF FAILED — " +
                                            error.javaClass.simpleName,
                                )
                            } finally {
                                PassphrasePolicy.wipe(passphraseBytes)
                            }

                        runOnUiThread {
                            clearSensitiveFields()
                            authorizationAcknowledgement?.isChecked = false
                            proofStatus.text = result.status
                            generateProof.isEnabled = prerequisites.ready
                            cancel.isEnabled = true

                            Toast.makeText(
                                this,
                                if (result.success) {
                                    "STAGE D PROOF PASS — NO TRANSACTION SUBMITTED"
                                } else {
                                    "STAGE D PROOF BLOCKED / FAILED"
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

    private fun loadPrerequisites(): ProofPrerequisites {
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
                return ProofPrerequisites(
                    ready = false,
                    expectedAddress = null,
                    lockedVaultJson = null,
                    status = "BLOCKED — DEVNET TRUST BOUNDARY NOT VERIFIED",
                )
            }

            val vaultStore = LockedVaultStore(this)
            if (!vaultStore.hasVault()) {
                return ProofPrerequisites(
                    ready = false,
                    expectedAddress = null,
                    lockedVaultJson = null,
                    status = "BLOCKED — ENCRYPTED WALLET VAULT MISSING",
                )
            }

            val lockedVaultJson =
                vaultStore.loadVault()
                    ?: return ProofPrerequisites(
                        ready = false,
                        expectedAddress = null,
                        lockedVaultJson = null,
                        status = "BLOCKED — ENCRYPTED WALLET VAULT UNREADABLE",
                    )

            val identityResult =
                NativeBridge.lockedVaultDevnetAddress(lockedVaultJson)

            if (!identityResult.startsWith("ok:")) {
                return ProofPrerequisites(
                    ready = false,
                    expectedAddress = null,
                    lockedVaultJson = null,
                    status = "BLOCKED — STORED WALLET IDENTITY NOT VERIFIED",
                )
            }

            val expectedAddress = identityResult.removePrefix("ok:")
            if (expectedAddress.isBlank()) {
                return ProofPrerequisites(
                    ready = false,
                    expectedAddress = null,
                    lockedVaultJson = null,
                    status = "BLOCKED — STORED WALLET IDENTITY IS EMPTY",
                )
            }

            ProofPrerequisites(
                ready = true,
                expectedAddress = expectedAddress,
                lockedVaultJson = lockedVaultJson,
                status = "READY — DEVNET BRIDGE, RPC, VAULT, AND IDENTITY VERIFIED",
            )
        } catch (error: Throwable) {
            ProofPrerequisites(
                ready = false,
                expectedAddress = null,
                lockedVaultJson = null,
                status =
                    "BLOCKED — PREREQUISITE CHECK FAILED: " +
                        error.javaClass.simpleName,
            )
        }
    }

    private fun runProof(
        expectedAddress: String,
        lockedVaultJson: String,
        passphraseBytes: ByteArray,
    ): ProofResult {
        val nativeResult =
            NativeBridge.signStageCDevnetProof(
                lockedVaultJson,
                passphraseBytes,
            )

        if (nativeResult == "wrong-passphrase") {
            return ProofResult(
                success = false,
                status = "PROOF BLOCKED — CURRENT PASSPHRASE NOT ACCEPTED",
            )
        }

        if (!nativeResult.startsWith("ok:")) {
            return ProofResult(
                success = false,
                status = "PROOF FAILED — $nativeResult",
            )
        }

        val parts = nativeResult.split(":", limit = 5)
        if (parts.size != 5 || parts[0] != "ok") {
            return ProofResult(
                success = false,
                status = "PROOF FAILED — INVALID NATIVE RESPONSE",
            )
        }

        val returnedAddress = parts[1]
        val signatureHex = parts[2]
        val recentBlockhash = parts[3]
        val reservedLamports = parts[4]

        if (returnedAddress != expectedAddress) {
            return ProofResult(
                success = false,
                status = "PROOF BLOCKED — WALLET IDENTITY MISMATCH",
            )
        }

        if (
            signatureHex.length != 128 ||
            !signatureHex.all { character ->
                character in '0'..'9' ||
                    character in 'a'..'f'
            }
        ) {
            return ProofResult(
                success = false,
                status = "PROOF FAILED — INVALID SIGNATURE ENCODING",
            )
        }

        if (recentBlockhash.isBlank()) {
            return ProofResult(
                success = false,
                status = "PROOF FAILED — RECENT BLOCKHASH MISSING",
            )
        }

        if (reservedLamports != "1") {
            return ProofResult(
                success = false,
                status = "PROOF BLOCKED — EXPOSURE RESERVATION CHANGED",
            )
        }

        return ProofResult(
            success = true,
            status =
                buildString {
                    append("STAGE D PHYSICAL PROOF — PASS")
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
                    append("Reserved lamports: ")
                    append(reservedLamports)
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
