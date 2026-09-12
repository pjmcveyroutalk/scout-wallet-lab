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

class CredentialRekeyActivity : Activity() {
    private var currentPassphraseField: EditText? = null
    private var newPassphraseField: EditText? = null
    private var confirmPassphraseField: EditText? = null
    private var recoveryAcknowledgement: CheckBox? = null

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

        fun securePassphraseField(hintText: String): EditText =
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

        root.addView(text("SCOUT PASSPHRASE RE-KEY", 24f))
        root.addView(text("DEVNET IDENTITY ONLY", 16f))
        root.addView(
            text(
                "This changes only the encryption passphrase around the existing Scout signing seed.",
                15f,
            ),
        )
        root.addView(
            text(
                "The public address must remain identical before and after the re-key.",
                15f,
            ),
        )
        root.addView(
            text(
                "NO SIGNING • NO TRANSACTION • NO MAINNET",
                14f,
            ),
        )

        val vaultStore = LockedVaultStore(this)
        val lockedVaultJson =
            try {
                vaultStore.loadVault()
            } catch (_: Throwable) {
                null
            }

        val expectedAddress =
            if (lockedVaultJson.isNullOrBlank()) {
                null
            } else {
                try {
                    NativeBridge.lockedVaultDevnetAddress(lockedVaultJson)
                        .takeIf { result -> result.startsWith("ok:") }
                        ?.removePrefix("ok:")
                        ?.takeIf { address -> address.isNotBlank() }
                } catch (_: Throwable) {
                    null
                }
            }

        root.addView(text("Existing Scout identity", 14f))
        root.addView(
            text(
                expectedAddress ?: "UNAVAILABLE — EXISTING VAULT NOT VERIFIED",
                16f,
            ),
        )

        val acknowledgement =
            CheckBox(this).apply {
                text = "I already verified and saved the 24-word Scout recovery copy for this identity."
                isSaveEnabled = false
                isEnabled = !expectedAddress.isNullOrBlank()
                contentDescription = "Confirm the verified Scout recovery copy exists before re-keying"
            }

        recoveryAcknowledgement = acknowledgement
        root.addView(
            acknowledgement,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        root.addView(text("Current wallet passphrase", 14f))
        val currentPassphrase =
            securePassphraseField("Enter current passphrase").apply {
                contentDescription = "Current Scout wallet passphrase"
            }
        currentPassphraseField = currentPassphrase
        root.addView(
            currentPassphrase,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        root.addView(text("New wallet passphrase", 14f))
        val newPassphrase =
            securePassphraseField(
                "Create new passphrase — at least ${PassphrasePolicy.MIN_LENGTH} characters",
            ).apply {
                contentDescription = "New Scout wallet passphrase"
            }
        newPassphraseField = newPassphrase
        root.addView(
            newPassphrase,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        root.addView(text("Confirm new wallet passphrase", 14f))
        val confirmPassphrase =
            securePassphraseField("Re-enter new passphrase").apply {
                contentDescription = "Confirm new Scout wallet passphrase"
            }
        confirmPassphraseField = confirmPassphrase
        root.addView(
            confirmPassphrase,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val status =
            text(
                if (!lockedVaultJson.isNullOrBlank() && !expectedAddress.isNullOrBlank()) {
                    "READY — RE-KEY REQUIRES EXPLICIT LOCAL CONFIRMATION"
                } else {
                    "BLOCKED — EXISTING ENCRYPTED VAULT NOT AVAILABLE"
                },
                18f,
            ).apply {
                gravity = Gravity.CENTER
                minHeight = padding * 4
            }

        root.addView(
            status,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val rekey =
            Button(this).apply {
                text = "RE-KEY EXISTING SCOUT VAULT"
                isEnabled =
                    !lockedVaultJson.isNullOrBlank() &&
                        !expectedAddress.isNullOrBlank()
                contentDescription = "Replace only the encrypted Scout vault passphrase"
            }

        fullWidth(rekey)
        root.addView(rekey)

        val cancel =
            Button(this).apply {
                text = "CANCEL — KEEP CURRENT VAULT"
                contentDescription = "Cancel Scout passphrase re-key"
            }

        fullWidth(cancel)
        root.addView(cancel)

        rekey.setOnClickListener {
            if (recoveryAcknowledgement?.isChecked != true) {
                status.text = "RE-KEY BLOCKED — VERIFY AND SAVE THE 24-WORD RECOVERY COPY FIRST"
                clearSensitiveFields()
                return@setOnClickListener
            }

            val currentText = currentPassphraseField?.text
            val newText = newPassphraseField?.text
            val confirmText = confirmPassphraseField?.text

            if (currentText == null || newText == null || confirmText == null) {
                status.text = "RE-KEY BLOCKED — PASSPHRASE INPUT UNAVAILABLE"
                clearSensitiveFields()
                return@setOnClickListener
            }

            val currentValidation =
                PassphrasePolicy.encodeCandidateForVerification(
                    currentText,
                )

            if (currentValidation is PassphrasePolicy.ValidationResult.Invalid) {
                status.text = "RE-KEY BLOCKED — ${currentValidation.reason}"
                clearSensitiveFields()
                return@setOnClickListener
            }

            val currentPassphraseBytes =
                (currentValidation as PassphrasePolicy.ValidationResult.Valid).passphraseBytes

            val newValidation =
                PassphrasePolicy.validateAndEncode(
                    newText,
                    confirmText,
                )

            if (newValidation is PassphrasePolicy.ValidationResult.Invalid) {
                PassphrasePolicy.wipe(currentPassphraseBytes)
                status.text = "RE-KEY BLOCKED — ${newValidation.reason}"
                clearSensitiveFields()
                return@setOnClickListener
            }

            val newPassphraseBytes =
                (newValidation as PassphrasePolicy.ValidationResult.Valid).passphraseBytes

            if (sameBytes(currentPassphraseBytes, newPassphraseBytes)) {
                PassphrasePolicy.wipe(currentPassphraseBytes)
                PassphrasePolicy.wipe(newPassphraseBytes)
                status.text = "RE-KEY BLOCKED — NEW PASSPHRASE MUST DIFFER FROM CURRENT PASSPHRASE"
                clearSensitiveFields()
                return@setOnClickListener
            }

            val originalVaultJson = lockedVaultJson.orEmpty()
            val address = expectedAddress.orEmpty()

            clearSensitiveFields()
            rekey.isEnabled = false
            cancel.isEnabled = false
            status.text = "VERIFYING BACKUP AND PREPARING LOCAL RE-KEY..."

            Thread {
                val result =
                    try {
                        performRekey(
                            vaultStore = vaultStore,
                            originalVaultJson = originalVaultJson,
                            expectedAddress = address,
                            currentPassphraseBytes = currentPassphraseBytes,
                            newPassphraseBytes = newPassphraseBytes,
                        )
                    } catch (error: Throwable) {
                        "RE-KEY BLOCKED — LOCAL RE-KEY FAILED (${error.javaClass.simpleName})"
                    } finally {
                        PassphrasePolicy.wipe(currentPassphraseBytes)
                        PassphrasePolicy.wipe(newPassphraseBytes)
                    }

                runOnUiThread {
                    status.text = result
                    rekey.isEnabled = true
                    cancel.isEnabled = true
                }
            }.start()
        }

        cancel.setOnClickListener {
            clearSensitiveFields()
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
        recoveryAcknowledgement?.isChecked = false
        super.onStop()
    }

    override fun onDestroy() {
        clearSensitiveFields()
        currentPassphraseField = null
        newPassphraseField = null
        confirmPassphraseField = null
        recoveryAcknowledgement = null
        super.onDestroy()
    }

    private fun performRekey(
        vaultStore: LockedVaultStore,
        originalVaultJson: String,
        expectedAddress: String,
        currentPassphraseBytes: ByteArray,
        newPassphraseBytes: ByteArray,
    ): String {
        if (originalVaultJson.isBlank() || expectedAddress.isBlank()) {
            return "RE-KEY BLOCKED — EXISTING VAULT IDENTITY UNAVAILABLE"
        }

        val backupResult =
            LockedVaultBackupManager.create(
                context = this,
                expectedAddress = expectedAddress,
            )

        if (!backupResult.success) {
            return "RE-KEY BLOCKED — ENCRYPTED BACKUP PRECHECK FAILED"
        }

        val nativeResult =
            NativeBridge.rekeyLockedDevnetVault(
                originalVaultJson,
                currentPassphraseBytes,
                newPassphraseBytes,
            )

        if (nativeResult == "wrong-passphrase") {
            return "NO MATCH — CURRENT PASSPHRASE NOT ACCEPTED"
        }

        val parts = nativeResult.split(':', limit = 3)

        if (parts.size != 3 || parts[0] != "ok") {
            return "RE-KEY BLOCKED — NATIVE RE-KEY FAILED"
        }

        val returnedAddress = parts[1]
        val replacementVaultJson = parts[2]

        if (
            returnedAddress != expectedAddress ||
            replacementVaultJson.isBlank()
        ) {
            return "RE-KEY BLOCKED — REPLACEMENT IDENTITY MISMATCH"
        }

        val replacementAddressResult =
            NativeBridge.lockedVaultDevnetAddress(
                replacementVaultJson,
            )

        if (replacementAddressResult != "ok:$expectedAddress") {
            return "RE-KEY BLOCKED — REPLACEMENT ADDRESS VERIFICATION FAILED"
        }

        val prewritePassphraseResult =
            NativeBridge.verifyLockedDevnetPassphrase(
                replacementVaultJson,
                newPassphraseBytes,
            )

        if (prewritePassphraseResult != "ok:$expectedAddress") {
            return "RE-KEY BLOCKED — NEW PASSPHRASE PREWRITE VERIFICATION FAILED"
        }

        if (
            !vaultStore.prepareRekeyReplacement(
                expectedCurrentVaultJson = originalVaultJson,
                replacementVaultJson = replacementVaultJson,
            )
        ) {
            return "RE-KEY BLOCKED — VAULT REPLACEMENT COMMIT FAILED"
        }

        val storedVaultJson = vaultStore.loadVault()

        if (storedVaultJson != replacementVaultJson) {
            val rolledBack = vaultStore.rollbackRekey(replacementVaultJson)

            return if (rolledBack) {
                "RE-KEY BLOCKED — STORAGE VERIFY FAILED; ORIGINAL VAULT RESTORED"
            } else {
                "RE-KEY STOPPED — STORAGE VERIFY FAILED AND ROLLBACK COULD NOT BE CONFIRMED"
            }
        }

        val storedAddressResult =
            NativeBridge.lockedVaultDevnetAddress(
                storedVaultJson,
            )
        val storedPassphraseResult =
            NativeBridge.verifyLockedDevnetPassphrase(
                storedVaultJson,
                newPassphraseBytes,
            )

        if (
            storedAddressResult != "ok:$expectedAddress" ||
            storedPassphraseResult != "ok:$expectedAddress"
        ) {
            val rolledBack = vaultStore.rollbackRekey(replacementVaultJson)

            return if (rolledBack) {
                "RE-KEY BLOCKED — POSTWRITE VERIFY FAILED; ORIGINAL VAULT RESTORED"
            } else {
                "RE-KEY STOPPED — POSTWRITE VERIFY FAILED AND ROLLBACK COULD NOT BE CONFIRMED"
            }
        }

        val finalized = vaultStore.finalizeRekey(replacementVaultJson)

        return if (finalized) {
            "PASS — PASSPHRASE RE-KEY COMPLETE\n\nSAME SCOUT IDENTITY\nAddress: $expectedAddress\n\nNO TRANSACTION WAS CREATED OR SUBMITTED"
        } else {
            "PASS — NEW PASSPHRASE ACTIVE\n\nSAME SCOUT IDENTITY\nAddress: $expectedAddress\n\nENCRYPTED ROLLBACK COPY RETAINED IN APP PRIVATE STORAGE"
        }
    }

    private fun clearSensitiveFields() {
        currentPassphraseField?.text?.clear()
        newPassphraseField?.text?.clear()
        confirmPassphraseField?.text?.clear()
    }

    private fun sameBytes(
        first: ByteArray,
        second: ByteArray,
    ): Boolean {
        if (first.size != second.size) {
            return false
        }

        var difference = 0

        for (index in first.indices) {
            difference =
                difference or
                    (first[index].toInt() xor second[index].toInt())
        }

        return difference == 0
    }
}
