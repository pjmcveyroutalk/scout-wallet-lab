package com.routalk.scoutoperator

import android.app.Activity
import android.content.Intent
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

class CredentialRecoveryActivity : Activity() {
    private var candidateField: EditText? = null

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

        root.addView(text("SCOUT CREDENTIAL RECOVERY", 24f))
        root.addView(text("DEVNET ONLY", 16f))
        root.addView(
            text(
                "This screen only checks whether a candidate passphrase can unlock the existing encrypted Scout vault.",
                16f,
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

        root.addView(text("Candidate passphrase", 14f))

        val candidate =
            EditText(this).apply {
                hint = "Enter a passphrase you may have used"
                inputType =
                    InputType.TYPE_CLASS_TEXT or
                        InputType.TYPE_TEXT_VARIATION_PASSWORD or
                        InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
                transformationMethod = PasswordTransformationMethod.getInstance()
                isSingleLine = true
                maxLines = 1
                contentDescription = "Candidate Scout wallet passphrase"

                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    importantForAutofill =
                        View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
                    setAutofillHints(null)
                    imeOptions =
                        imeOptions or
                            EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING
                }
            }

        candidateField = candidate

        root.addView(
            candidate,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val status =
            text(
                if (
                    !lockedVaultJson.isNullOrBlank() &&
                    !expectedAddress.isNullOrBlank()
                ) {
                    "READY — LOCAL PASSPHRASE VERIFICATION"
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

        val verify =
            Button(this).apply {
                text = "VERIFY CURRENT PASSPHRASE"
                isEnabled =
                    !lockedVaultJson.isNullOrBlank() &&
                        !expectedAddress.isNullOrBlank()
                contentDescription =
                    "Verify a candidate passphrase against the existing encrypted Scout vault"
            }

        fullWidth(verify)
        root.addView(verify)

        val returnToScout =
            Button(this).apply {
                text = "RETURN TO SCOUT"
                contentDescription = "Return to the Scout Operator wallet screen"
            }

        fullWidth(returnToScout)
        root.addView(returnToScout)

        verify.setOnClickListener {
            val candidateText = candidateField?.text

            if (candidateText == null) {
                status.text = "VERIFICATION BLOCKED — PASSPHRASE INPUT UNAVAILABLE"
                clearSensitiveField()
                return@setOnClickListener
            }

            when (
                val validation =
                    PassphrasePolicy.validateAndEncode(
                        candidateText,
                        candidateText,
                    )
            ) {
                is PassphrasePolicy.ValidationResult.Invalid -> {
                    status.text =
                        "VERIFICATION BLOCKED — ${validation.reason}"
                    clearSensitiveField()
                }

                is PassphrasePolicy.ValidationResult.Valid -> {
                    val passphraseBytes = validation.passphraseBytes
                    val vaultJson = lockedVaultJson.orEmpty()
                    val address = expectedAddress.orEmpty()

                    clearSensitiveField()
                    verify.isEnabled = false
                    status.text = "VERIFYING LOCALLY..."

                    Thread {
                        val result =
                            try {
                                NativeBridge.verifyLockedDevnetPassphrase(
                                    vaultJson,
                                    passphraseBytes,
                                )
                            } catch (error: Throwable) {
                                "verification-bridge-failed:${error.javaClass.simpleName}"
                            } finally {
                                PassphrasePolicy.wipe(passphraseBytes)
                            }

                        runOnUiThread {
                            status.text =
                                when {
                                    result == "wrong-passphrase" ->
                                        "NO MATCH — TRY ANOTHER CANDIDATE"
                                    result == "ok:$address" ->
                                        "PASS — CURRENT PASSPHRASE VERIFIED\n\nAddress: $address"
                                    result.startsWith("ok:") ->
                                        "VERIFICATION BLOCKED — IDENTITY MISMATCH"
                                    else ->
                                        "VERIFICATION BLOCKED — $result"
                                }

                            verify.isEnabled = true
                        }
                    }.start()
                }
            }
        }

        returnToScout.setOnClickListener {
            clearSensitiveField()
            startActivity(Intent(this, MainActivity::class.java))
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
        clearSensitiveField()
        super.onStop()
    }

    override fun onDestroy() {
        clearSensitiveField()
        candidateField = null
        super.onDestroy()
    }

    private fun clearSensitiveField() {
        candidateField?.text?.clear()
    }
}
