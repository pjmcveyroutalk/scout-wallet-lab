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

class RecoveryWordsActivity : Activity() {
    private var passphraseField: EditText? = null
    private var recoveryWordsView: TextView? = null
    private var recoveryCopyField: EditText? = null
    private var statusView: TextView? = null
    private var hideAndVerifyButton: Button? = null
    private var verifyCopyButton: Button? = null

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

        val root =
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_HORIZONTAL
                setPadding(padding, padding, padding, padding)
            }

        root.addView(text("SCOUT EMERGENCY RECOVERY WORDS", 24f))
        root.addView(text("DEVNET WALLET IDENTITY ONLY", 16f))
        root.addView(
            text(
                "These 24 words reconstruct the existing Scout signing seed. They are NOT a standard Solana HD recovery phrase.",
                15f,
            ),
        )
        root.addView(
            text(
                "Keep them offline. Never paste them into a website, browser, chat, email, or cloud note.",
                15f,
            ),
        )
        root.addView(
            text(
                "NO SIGNING • NO TRANSACTION • NO VAULT CHANGES",
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

        root.addView(text("Current wallet passphrase", 14f))

        val passphrase =
            EditText(this).apply {
                hint = "Enter current passphrase to reveal words"
                inputType =
                    InputType.TYPE_CLASS_TEXT or
                        InputType.TYPE_TEXT_VARIATION_PASSWORD or
                        InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
                transformationMethod = PasswordTransformationMethod.getInstance()
                isSingleLine = true
                maxLines = 1
                isSaveEnabled = false
                contentDescription = "Current Scout wallet passphrase"

                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    importantForAutofill =
                        View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
                    setAutofillHints(null)
                    imeOptions =
                        imeOptions or
                            EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING
                }
            }

        passphraseField = passphrase

        root.addView(
            passphrase,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val reveal =
            Button(this).apply {
                text = "REVEAL 24 RECOVERY WORDS"
                isEnabled =
                    !lockedVaultJson.isNullOrBlank() &&
                        !expectedAddress.isNullOrBlank()
                contentDescription = "Reveal Scout emergency recovery words after local passphrase check"
            }

        fullWidth(reveal)
        root.addView(reveal)

        val words =
            text("RECOVERY WORDS HIDDEN", 18f).apply {
                gravity = Gravity.START
                isSaveEnabled = false
                setTextIsSelectable(false)
            }

        recoveryWordsView = words

        root.addView(
            words,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val hideAndVerify =
            Button(this).apply {
                text = "I SAVED THEM — HIDE & VERIFY COPY"
                isEnabled = false
                contentDescription = "Hide the displayed recovery words and verify the saved copy"
            }

        hideAndVerifyButton = hideAndVerify
        fullWidth(hideAndVerify)
        root.addView(hideAndVerify)

        val recoveryCopy =
            EditText(this).apply {
                hint = "Enter your saved 24-word copy"
                inputType =
                    InputType.TYPE_CLASS_TEXT or
                        InputType.TYPE_TEXT_FLAG_MULTI_LINE or
                        InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
                minLines = 4
                maxLines = 8
                isSaveEnabled = false
                visibility = View.GONE
                contentDescription = "Saved Scout recovery words"

                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    importantForAutofill =
                        View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
                    setAutofillHints(null)
                    imeOptions =
                        imeOptions or
                            EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING
                }
            }

        recoveryCopyField = recoveryCopy

        root.addView(
            recoveryCopy,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val verifyCopy =
            Button(this).apply {
                text = "VERIFY SAVED RECOVERY COPY"
                visibility = View.GONE
                contentDescription = "Verify that saved recovery words reconstruct the current Scout identity"
            }

        verifyCopyButton = verifyCopy
        fullWidth(verifyCopy)
        root.addView(verifyCopy)

        val status =
            text(
                if (
                    !lockedVaultJson.isNullOrBlank() &&
                    !expectedAddress.isNullOrBlank()
                ) {
                    "READY — LOCAL RECOVERY BACKUP"
                } else {
                    "BLOCKED — EXISTING ENCRYPTED VAULT NOT AVAILABLE"
                },
                18f,
            ).apply {
                gravity = Gravity.CENTER
                minHeight = padding * 4
            }

        statusView = status

        root.addView(
            status,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val returnButton =
            Button(this).apply {
                text = "RETURN TO CREDENTIAL RECOVERY"
                contentDescription = "Return to Scout credential recovery"
            }

        fullWidth(returnButton)
        root.addView(returnButton)

        reveal.setOnClickListener {
            val passphraseText = passphraseField?.text

            if (passphraseText == null) {
                status.text = "RECOVERY BLOCKED — PASSPHRASE INPUT UNAVAILABLE"
                clearSensitiveInputs()
                return@setOnClickListener
            }

            when (
                val validation =
                    PassphrasePolicy.encodeCandidateForVerification(
                        passphraseText,
                    )
            ) {
                is PassphrasePolicy.ValidationResult.Invalid -> {
                    status.text = "RECOVERY BLOCKED — ${validation.reason}"
                    clearSensitiveInputs()
                }

                is PassphrasePolicy.ValidationResult.Valid -> {
                    val passphraseBytes = validation.passphraseBytes
                    val vaultJson = lockedVaultJson.orEmpty()
                    val address = expectedAddress.orEmpty()

                    passphraseField?.text?.clear()
                    reveal.isEnabled = false
                    hideAndVerify.isEnabled = false
                    words.text = "RECOVERY WORDS HIDDEN"
                    status.text = "DERIVING RECOVERY WORDS LOCALLY..."

                    Thread {
                        val result =
                            try {
                                NativeBridge.exportLockedVaultRecoveryWords(
                                    vaultJson,
                                    passphraseBytes,
                                )
                            } catch (error: Throwable) {
                                "recovery-bridge-failed:${error.javaClass.simpleName}"
                            } finally {
                                PassphrasePolicy.wipe(passphraseBytes)
                            }

                        runOnUiThread {
                            val parts = result.split(':', limit = 3)

                            when {
                                result == "wrong-passphrase" -> {
                                    status.text = "NO MATCH — CURRENT PASSPHRASE NOT ACCEPTED"
                                    words.text = "RECOVERY WORDS HIDDEN"
                                    hideAndVerify.isEnabled = false
                                }

                                parts.size != 3 || parts[0] != "ok" -> {
                                    status.text = "RECOVERY BLOCKED — LOCAL EXPORT FAILED"
                                    words.text = "RECOVERY WORDS HIDDEN"
                                    hideAndVerify.isEnabled = false
                                }

                                parts[1] != address -> {
                                    status.text = "RECOVERY BLOCKED — IDENTITY MISMATCH"
                                    words.text = "RECOVERY WORDS HIDDEN"
                                    hideAndVerify.isEnabled = false
                                }

                                else -> {
                                    val recoveredWords =
                                        parts[2]
                                            .trim()
                                            .split(Regex("\\s+"))
                                            .filter { word -> word.isNotBlank() }

                                    if (recoveredWords.size != RECOVERY_WORD_COUNT) {
                                        status.text = "RECOVERY BLOCKED — WORD COUNT MISMATCH"
                                        words.text = "RECOVERY WORDS HIDDEN"
                                        hideAndVerify.isEnabled = false
                                    } else {
                                        words.text =
                                            recoveredWords
                                                .mapIndexed { index, word -> "${index + 1}. $word" }
                                                .joinToString("\n")
                                        status.text =
                                            "24 WORDS REVEALED — RECORD THEM OFFLINE BEFORE CONTINUING"
                                        hideAndVerify.isEnabled = true
                                    }
                                }
                            }

                            reveal.isEnabled = true
                        }
                    }.start()
                }
            }
        }

        hideAndVerify.setOnClickListener {
            words.text = "RECOVERY WORDS HIDDEN"
            hideAndVerify.isEnabled = false
            recoveryCopy.visibility = View.VISIBLE
            verifyCopy.visibility = View.VISIBLE
            status.text = "WORDS HIDDEN — ENTER YOUR SAVED 24-WORD COPY"
            recoveryCopy.requestFocus()
        }

        verifyCopy.setOnClickListener {
            val enteredWords = recoveryCopyField?.text?.toString()?.trim().orEmpty()

            if (enteredWords.isBlank()) {
                status.text = "VERIFICATION BLOCKED — RECOVERY WORDS REQUIRED"
                recoveryCopyField?.text?.clear()
                return@setOnClickListener
            }

            val wordCount = enteredWords.split(Regex("\\s+")).count { word -> word.isNotBlank() }

            if (wordCount != RECOVERY_WORD_COUNT) {
                status.text = "VERIFICATION BLOCKED — EXACTLY 24 WORDS REQUIRED"
                recoveryCopyField?.text?.clear()
                return@setOnClickListener
            }

            val vaultJson = lockedVaultJson.orEmpty()
            val address = expectedAddress.orEmpty()

            recoveryCopyField?.text?.clear()
            verifyCopy.isEnabled = false
            status.text = "VERIFYING SAVED COPY LOCALLY..."

            Thread {
                val result =
                    try {
                        NativeBridge.verifyLockedVaultRecoveryWords(
                            vaultJson,
                            enteredWords,
                        )
                    } catch (error: Throwable) {
                        "recovery-verification-bridge-failed:${error.javaClass.simpleName}"
                    }

                runOnUiThread {
                    status.text =
                        when {
                            result == "ok:$address" ->
                                "RECOVERY COPY VERIFIED — SAME SCOUT IDENTITY\n\nAddress: $address"
                            result.startsWith("ok:") ->
                                "VERIFICATION BLOCKED — IDENTITY MISMATCH"
                            result == "recovery-identity-mismatch" ->
                                "NO MATCH — SAVED WORDS DO NOT RECONSTRUCT THIS SCOUT IDENTITY"
                            result == "recovery-words-invalid" ||
                                result == "invalid-recovery-word-count" ->
                                "NO MATCH — SAVED RECOVERY WORDS ARE INVALID"
                            else ->
                                "VERIFICATION BLOCKED — LOCAL RECOVERY CHECK FAILED"
                        }

                    verifyCopy.isEnabled = true
                }
            }.start()
        }

        returnButton.setOnClickListener {
            clearSensitiveInputs()
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
        clearSensitiveInputs()
        statusView?.text = "RECOVERY WORDS HIDDEN — REAUTHENTICATE TO REVEAL"
        super.onStop()
    }

    override fun onDestroy() {
        clearSensitiveInputs()
        passphraseField = null
        recoveryWordsView = null
        recoveryCopyField = null
        statusView = null
        hideAndVerifyButton = null
        verifyCopyButton = null
        super.onDestroy()
    }

    private fun clearSensitiveInputs() {
        passphraseField?.text?.clear()
        recoveryCopyField?.text?.clear()
        recoveryWordsView?.text = "RECOVERY WORDS HIDDEN"
        hideAndVerifyButton?.isEnabled = false
        recoveryCopyField?.visibility = View.GONE
        verifyCopyButton?.visibility = View.GONE
    }

    companion object {
        private const val RECOVERY_WORD_COUNT = 24
    }
}
