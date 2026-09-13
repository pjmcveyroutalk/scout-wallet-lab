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

class StageFPresubmitActivity : Activity() {
    private var passphraseField: EditText? = null
    private var confirmationField: EditText? = null
    private var authorizationAcknowledgement: CheckBox? = null
    private var submissionAuthorizationAcknowledgement: CheckBox? = null
    private var preparedCandidateToken: String? = null
    private var preparedPublicReviewSnapshot: StageFBPublicReviewSnapshot.Snapshot? = null

    private data class PresubmitPrerequisites(
        val ready: Boolean,
        val expectedAddress: String?,
        val lockedVaultJson: String?,
        val status: String,
    )

    private data class PresubmitResult(
        val success: Boolean,
        val status: String,
        val candidateToken: String? = null,
        val publicReviewSnapshot: StageFBPublicReviewSnapshot.Snapshot? = null,
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
                    importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
                    setAutofillHints(null)
                    imeOptions = imeOptions or EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING
                }
            }

        val root =
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_HORIZONTAL
                setPadding(padding, padding, padding, padding)
            }

        root.addView(text("SCOUT STAGE F DEVNET PROOF", 24f))
        root.addView(text("DEVNET ONLY", 18f))
        root.addView(text("STAGE F-B PHYSICAL SEND — NOT ARMED", 14f))
        root.addView(text("MAINNET — DISABLED", 14f))
        root.addView(text("ARBITRARY SIGNING — DISABLED", 14f))
        root.addView(
            text(
                "Stage F-A prepares, signs, and simulates only the fixed Scout Devnet Memo candidate. Stage F-B has a guarded one-shot submission implementation, but the physical send remains hard-disabled in this build.",
                14f,
            ),
        )
        root.addView(
            text(
                "Fixed payload: scout-stage-f-devnet-submission-proof-v1",
                13f,
            ),
        )
        root.addView(text("Maximum accepted Devnet fee: 10,000 lamports", 13f))
        root.addView(text("Minimum remaining Devnet balance: 1,000,000 lamports", 13f))

        val prerequisites = loadPrerequisites()

        root.addView(text("Presubmit readiness", 14f))
        root.addView(text(prerequisites.status, 16f))
        root.addView(text("Verified wallet address", 14f))
        root.addView(text(prerequisites.expectedAddress ?: "Unavailable", 16f))

        val authorization =
            CheckBox(this).apply {
                text =
                    "I authorize this local Stage F-A signing + simulation presubmit proof. " +
                        "No transaction will be submitted by this authorization."
                isSaveEnabled = false
                isEnabled = prerequisites.ready
                contentDescription = "Authorize the fixed Scout Stage F-A Devnet presubmit proof"
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
                description = "Stage F-A Scout wallet passphrase",
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
                description = "Confirm Stage F-A Scout wallet passphrase",
            )
        confirmationField = confirmation
        root.addView(
            confirmation,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val presubmitStatus =
            text(
                if (prerequisites.ready) {
                    "READY — EXPLICIT LOCAL PRESUBMIT AUTHORIZATION REQUIRED"
                } else {
                    "BLOCKED — PREREQUISITES NOT SATISFIED"
                },
                16f,
            ).apply {
                gravity = Gravity.CENTER
                minHeight = padding * 4
            }
        root.addView(
            presubmitStatus,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val prepare =
            Button(this).apply {
                text = "PREPARE STAGE F-A PRESUBMIT CANDIDATE"
                isEnabled = prerequisites.ready
                contentDescription =
                    "Prepare sign and simulate the fixed Scout Stage F-A Devnet candidate without submitting it"
            }
        fullWidth(prepare)
        root.addView(prepare)

        val discard =
            Button(this).apply {
                text = "DISCARD PREPARED CANDIDATE"
                isEnabled = false
                contentDescription = "Discard the in-memory Scout Stage F-A candidate"
            }
        fullWidth(discard)
        root.addView(discard)

        val submissionAuthorization =
            CheckBox(this).apply {
                text =
                    "I explicitly authorize exactly one Stage F-B Devnet submission attempt for " +
                        "the reviewed fixed candidate. No retry or rebroadcast is permitted."
                isSaveEnabled = false
                isEnabled = false
                contentDescription = "Authorize exactly one fixed Scout Stage F-B Devnet submission attempt"
            }
        submissionAuthorizationAcknowledgement = submissionAuthorization
        root.addView(
            submissionAuthorization,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val submit =
            Button(this).apply {
                text = "STAGE F-B ONE-SHOT DEVNET SUBMISSION — NOT ARMED"
                isEnabled = false
                contentDescription = "Submit the reviewed fixed Stage F-B Devnet candidate exactly once"
            }
        fullWidth(submit)
        root.addView(submit)

        val cancel =
            Button(this).apply {
                text = "CANCEL — DO NOT PREPARE OR SUBMIT"
                contentDescription = "Cancel the Scout Stage F proof"
            }
        fullWidth(cancel)
        root.addView(cancel)

        submissionAuthorization.setOnCheckedChangeListener { _, checked ->
            submit.isEnabled =
                STAGE_FB_PHYSICAL_SEND_ARMED &&
                    checked &&
                    preparedCandidateToken != null &&
                    preparedPublicReviewSnapshot != null &&
                    StageFBAttemptGuard(this).load() is StageFBAttemptGuard.LoadResult.Empty
        }

        prepare.setOnClickListener {
            if (preparedCandidateToken != null) {
                presubmitStatus.text = "BLOCKED — DISCARD THE EXISTING CANDIDATE FIRST"
                clearSensitiveFields()
                return@setOnClickListener
            }

            if (authorizationAcknowledgement?.isChecked != true) {
                presubmitStatus.text =
                    "BLOCKED — EXPLICIT LOCAL STAGE F-A PRESUBMIT AUTHORIZATION REQUIRED"
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
                presubmitStatus.text =
                    "BLOCKED — PRESUBMIT PREREQUISITES BECAME UNAVAILABLE"
                clearSensitiveFields()
                prepare.isEnabled = false
                return@setOnClickListener
            }

            if (currentPassphrase.toString() != currentConfirmation.toString()) {
                presubmitStatus.text =
                    "BLOCKED — CURRENT PASSPHRASE CONFIRMATION DOES NOT MATCH"
                clearSensitiveFields()
                return@setOnClickListener
            }

            when (
                val validation = PassphrasePolicy.encodeCandidateForVerification(currentPassphrase)
            ) {
                is PassphrasePolicy.ValidationResult.Invalid -> {
                    presubmitStatus.text = "BLOCKED — ${validation.reason}"
                    clearSensitiveFields()
                }

                is PassphrasePolicy.ValidationResult.Valid -> {
                    val passphraseBytes = validation.passphraseBytes
                    clearSensitiveFields()
                    prepare.isEnabled = false
                    discard.isEnabled = false
                    submit.isEnabled = false
                    submissionAuthorization.isEnabled = false
                    cancel.isEnabled = false
                    presubmitStatus.text =
                        "PREPARING FIXED CANDIDATE AND RUNNING DEVNET SIMULATION..."

                    Thread {
                        val result =
                            try {
                                runPresubmit(
                                    expectedAddress = expectedAddress,
                                    lockedVaultJson = lockedVaultJson,
                                    passphraseBytes = passphraseBytes,
                                )
                            } catch (error: Throwable) {
                                PresubmitResult(
                                    success = false,
                                    status = "PRESUBMIT FAILED — ${error.javaClass.simpleName}",
                                )
                            } finally {
                                PassphrasePolicy.wipe(passphraseBytes)
                            }

                        runOnUiThread {
                            clearSensitiveFields()
                            authorizationAcknowledgement?.isChecked = false
                            preparedCandidateToken = result.candidateToken
                            preparedPublicReviewSnapshot = result.publicReviewSnapshot
                            presubmitStatus.text = result.status
                            prepare.isEnabled = prerequisites.ready && !result.success
                            discard.isEnabled = result.success
                            submissionAuthorization.isChecked = false
                            submissionAuthorization.isEnabled =
                                result.success && STAGE_FB_PHYSICAL_SEND_ARMED
                            submit.isEnabled = false
                            cancel.isEnabled = true

                            Toast.makeText(
                                this,
                                if (result.success) {
                                    "STAGE F-A PRESUBMIT PASS — PHYSICAL SEND NOT ARMED"
                                } else {
                                    "STAGE F-A PRESUBMIT BLOCKED / FAILED"
                                },
                                Toast.LENGTH_LONG,
                            ).show()
                        }
                    }.start()
                }
            }
        }

        discard.setOnClickListener {
            if (discardPreparedCandidate()) {
                presubmitStatus.text =
                    "STAGE F-A CANDIDATE DISCARDED — NOTHING SUBMITTED"
                discard.isEnabled = false
                submissionAuthorization.isChecked = false
                submissionAuthorization.isEnabled = false
                submit.isEnabled = false
                prepare.isEnabled =
                    prerequisites.ready &&
                        StageFBAttemptGuard(this).load() is StageFBAttemptGuard.LoadResult.Empty
            } else {
                presubmitStatus.text =
                    "DISCARD FAILED — CANDIDATE REMAINS IN MEMORY; DO NOT CONTINUE"
                prepare.isEnabled = false
                submissionAuthorization.isEnabled = false
                submit.isEnabled = false
            }
        }

        submit.setOnClickListener {
            if (!STAGE_FB_PHYSICAL_SEND_ARMED) {
                presubmitStatus.text = "BLOCKED — STAGE F-B PHYSICAL SEND IS NOT ARMED"
                submit.isEnabled = false
                return@setOnClickListener
            }

            if (submissionAuthorizationAcknowledgement?.isChecked != true) {
                presubmitStatus.text =
                    "BLOCKED — EXPLICIT ONE-SHOT STAGE F-B SUBMISSION AUTHORIZATION REQUIRED"
                return@setOnClickListener
            }

            val token = preparedCandidateToken
            val snapshot = preparedPublicReviewSnapshot
            if (token == null || snapshot == null) {
                presubmitStatus.text = "BLOCKED — REVIEWED CANDIDATE IS NOT AVAILABLE"
                submit.isEnabled = false
                return@setOnClickListener
            }

            submit.isEnabled = false
            discard.isEnabled = false
            prepare.isEnabled = false
            cancel.isEnabled = false
            submissionAuthorization.isEnabled = false
            presubmitStatus.text =
                "STAGE F-B — PERSISTING ONE-ATTEMPT GUARD BEFORE ANY LEDGER WRITE..."

            Thread {
                val outcome = runStageFBSubmission(token, snapshot)

                runOnUiThread {
                    preparedCandidateToken = null
                    preparedPublicReviewSnapshot = null
                    submissionAuthorizationAcknowledgement?.isChecked = false
                    submissionAuthorizationAcknowledgement?.isEnabled = false
                    submit.isEnabled = false
                    discard.isEnabled = false
                    prepare.isEnabled =
                        prerequisites.ready &&
                            StageFBAttemptGuard(this).load() is StageFBAttemptGuard.LoadResult.Empty
                    cancel.isEnabled = true
                    presubmitStatus.text = outcome

                    Toast.makeText(
                        this,
                        "STAGE F-B ATTEMPT TERMINAL — USE READ-ONLY RESOLUTION",
                        Toast.LENGTH_LONG,
                    ).show()
                }
            }.start()
        }

        cancel.setOnClickListener {
            discardPreparedCandidate()
            clearSensitiveFields()
            authorizationAcknowledgement?.isChecked = false
            submissionAuthorizationAcknowledgement?.isChecked = false
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
        discardPreparedCandidate()
        clearSensitiveFields()
        authorizationAcknowledgement?.isChecked = false
        submissionAuthorizationAcknowledgement?.isChecked = false
        super.onStop()
    }

    override fun onDestroy() {
        discardPreparedCandidate()
        clearSensitiveFields()
        passphraseField = null
        confirmationField = null
        authorizationAcknowledgement = null
        submissionAuthorizationAcknowledgement = null
        preparedCandidateToken = null
        preparedPublicReviewSnapshot = null
        super.onDestroy()
    }

    private fun loadPrerequisites(): PresubmitPrerequisites {
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
                return PresubmitPrerequisites(
                    ready = false,
                    expectedAddress = null,
                    lockedVaultJson = null,
                    status = "BLOCKED — DEVNET TRUST BOUNDARY NOT VERIFIED",
                )
            }

            val vaultStore = LockedVaultStore(this)
            if (!vaultStore.hasVault()) {
                return PresubmitPrerequisites(
                    ready = false,
                    expectedAddress = null,
                    lockedVaultJson = null,
                    status = "BLOCKED — ENCRYPTED WALLET VAULT MISSING",
                )
            }

            val lockedVaultJson =
                vaultStore.loadVault()
                    ?: return PresubmitPrerequisites(
                        ready = false,
                        expectedAddress = null,
                        lockedVaultJson = null,
                        status = "BLOCKED — ENCRYPTED WALLET VAULT UNREADABLE",
                    )

            val identityResult = NativeBridge.lockedVaultDevnetAddress(lockedVaultJson)
            if (!identityResult.startsWith("ok:")) {
                return PresubmitPrerequisites(
                    ready = false,
                    expectedAddress = null,
                    lockedVaultJson = null,
                    status = "BLOCKED — STORED WALLET IDENTITY NOT VERIFIED",
                )
            }

            val expectedAddress = identityResult.removePrefix("ok:")
            if (expectedAddress.isBlank()) {
                return PresubmitPrerequisites(
                    ready = false,
                    expectedAddress = null,
                    lockedVaultJson = null,
                    status = "BLOCKED — STORED WALLET IDENTITY IS EMPTY",
                )
            }

            PresubmitPrerequisites(
                ready = true,
                expectedAddress = expectedAddress,
                lockedVaultJson = lockedVaultJson,
                status = "READY — DEVNET BRIDGE, RPC, VAULT, AND IDENTITY VERIFIED",
            )
        } catch (error: Throwable) {
            PresubmitPrerequisites(
                ready = false,
                expectedAddress = null,
                lockedVaultJson = null,
                status = "BLOCKED — PREREQUISITE CHECK FAILED: ${error.javaClass.simpleName}",
            )
        }
    }

    private fun runPresubmit(
        expectedAddress: String,
        lockedVaultJson: String,
        passphraseBytes: ByteArray,
    ): PresubmitResult {
        val nativeResult =
            NativeBridge.prepareStageFDevnetCandidate(
                lockedVaultJson,
                passphraseBytes,
            )

        if (!nativeResult.startsWith("ok:")) {
            return PresubmitResult(
                success = false,
                status = "PRESUBMIT FAILED — $nativeResult",
            )
        }

        val parts = nativeResult.split(":", limit = 11)
        if (parts.size != 11 || parts[0] != "ok") {
            return PresubmitResult(
                success = false,
                status = "PRESUBMIT FAILED — INVALID NATIVE RESPONSE",
            )
        }

        val returnedAddress = parts[1]
        val signatureHex = parts[2]
        val recentBlockhash = parts[3]
        val feeLamports = parts[4].toLongOrNull()
        val balanceLamports = parts[5].toLongOrNull()
        val remainingBalanceLamports = parts[6].toLongOrNull()
        val simulationSlot = parts[7].toLongOrNull()
        val unitsConsumed = parts[8].toLongOrNull()
        val lastValidBlockHeight = parts[9].toLongOrNull()
        val candidateToken = parts[10]

        if (returnedAddress != expectedAddress) {
            return PresubmitResult(false, "PRESUBMIT BLOCKED — WALLET IDENTITY MISMATCH")
        }

        if (
            signatureHex.length != 128 ||
            !signatureHex.all { character -> character in '0'..'9' || character in 'a'..'f' }
        ) {
            return PresubmitResult(false, "PRESUBMIT FAILED — INVALID SIGNATURE ENCODING")
        }

        if (recentBlockhash.isBlank()) {
            return PresubmitResult(false, "PRESUBMIT FAILED — RECENT BLOCKHASH MISSING")
        }

        if (feeLamports == null || feeLamports <= 0L || feeLamports > 10_000L) {
            return PresubmitResult(false, "PRESUBMIT BLOCKED — DEVNET FEE OUTSIDE FIXED BOUNDARY")
        }

        if (
            balanceLamports == null ||
            remainingBalanceLamports == null ||
            remainingBalanceLamports < 1_000_000L ||
            balanceLamports - feeLamports != remainingBalanceLamports
        ) {
            return PresubmitResult(false, "PRESUBMIT BLOCKED — DEVNET BALANCE FLOOR INVALID")
        }

        if (simulationSlot == null || simulationSlot <= 0L) {
            return PresubmitResult(false, "PRESUBMIT FAILED — SIMULATION SLOT INVALID")
        }

        if (unitsConsumed == null || unitsConsumed <= 0L) {
            return PresubmitResult(false, "PRESUBMIT FAILED — SIMULATION UNITS INVALID")
        }

        if (lastValidBlockHeight == null || lastValidBlockHeight <= 0L) {
            return PresubmitResult(false, "PRESUBMIT FAILED — BLOCKHASH LEASE INVALID")
        }

        if (
            candidateToken.length != 32 ||
            !candidateToken.all { character -> character in '0'..'9' || character in 'a'..'f' }
        ) {
            return PresubmitResult(false, "PRESUBMIT FAILED — CANDIDATE TOKEN INVALID")
        }

        val publicReview =
            StageFBPublicReviewSnapshot.createFromPublicPresubmit(
                scoutPublicKey = returnedAddress,
                signatureHex = signatureHex,
                recentBlockhash = recentBlockhash,
                feeLamports = feeLamports,
                balanceLamports = balanceLamports,
                remainingBalanceLamports = remainingBalanceLamports,
                simulationSlot = simulationSlot,
                unitsConsumed = unitsConsumed,
                lastValidBlockHeight = lastValidBlockHeight,
            )
        if (publicReview !is StageFBPublicReviewSnapshot.Result.Valid) {
            return PresubmitResult(
                success = false,
                status = "PRESUBMIT FAILED — STAGE F-B PUBLIC REVIEW SNAPSHOT INVALID",
            )
        }

        val publicReviewSnapshot = publicReview.snapshot

        return PresubmitResult(
            success = true,
            candidateToken = candidateToken,
            publicReviewSnapshot = publicReviewSnapshot,
            status =
                buildString {
                    append("STAGE F-A PRESUBMIT PROOF — PASS")
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
                    append("Fee: ")
                    append(feeLamports)
                    append(" lamports")
                    append("\n")
                    append("Balance before fee: ")
                    append(balanceLamports)
                    append(" lamports")
                    append("\n")
                    append("Balance after fee floor check: ")
                    append(remainingBalanceLamports)
                    append(" lamports")
                    append("\n")
                    append("Simulation slot: ")
                    append(simulationSlot)
                    append("\n")
                    append("Units consumed: ")
                    append(unitsConsumed)
                    append("\n")
                    append("Last valid block height: ")
                    append(lastValidBlockHeight)
                    append("\n\n")
                    append("STAGE F-B PUBLIC REVIEW SNAPSHOT — VERIFIED")
                    append("\n")
                    append("Public signature: ")
                    append(publicReviewSnapshot.metadata.expectedSignature)
                    append("\n")
                    append("Candidate fingerprint SHA-256: ")
                    append(publicReviewSnapshot.candidateFingerprintSha256)
                    append("\n")
                    append("Review receipt SHA-256: ")
                    append(publicReviewSnapshot.reviewReceiptSha256)
                    append("\n")
                    append("EXECUTION AUTHORIZED: NO")
                    append("\n\n")
                    append("IN-MEMORY CANDIDATE: HELD — NOT SUBMITTED")
                    append("\n")
                    append("STAGE F-B IMPLEMENTATION: READY")
                    append("\n")
                    append("PHYSICAL DEVNET SEND: NOT ARMED")
                    append("\n")
                    append("MAINNET: DISABLED")
                    append("\n")
                    append("ARBITRARY SIGNING: DISABLED")
                },
        )
    }

    private fun runStageFBSubmission(
        token: String,
        snapshot: StageFBPublicReviewSnapshot.Snapshot,
    ): String {
        try {
            if (!STAGE_FB_PHYSICAL_SEND_ARMED) {
                return "BLOCKED — STAGE F-B PHYSICAL SEND IS NOT ARMED"
            }

            val guard = StageFBAttemptGuard(this)
            if (guard.load() !is StageFBAttemptGuard.LoadResult.Empty) {
                return "BLOCKED — ONE-ATTEMPT GUARD IS ALREADY PRESENT; DO NOT RETRY"
            }

            val blockHeightResult = NativeBridge.devnetBlockHeight()
            if (!blockHeightResult.startsWith("ok:")) {
                return "BLOCKED — CURRENT DEVNET BLOCK HEIGHT UNAVAILABLE; NOTHING SUBMITTED"
            }

            val currentBlockHeight = blockHeightResult.removePrefix("ok:").toLongOrNull()
            if (currentBlockHeight == null || currentBlockHeight <= 0L) {
                return "BLOCKED — CURRENT DEVNET BLOCK HEIGHT INVALID; NOTHING SUBMITTED"
            }

            if (currentBlockHeight > snapshot.metadata.lastValidBlockHeight) {
                return "BLOCKED — REVIEWED CANDIDATE EXPIRED; NOTHING SUBMITTED"
            }

            when (
                guard.beginAttempt(
                    candidateFingerprintSha256 = snapshot.candidateFingerprintSha256,
                    reviewReceiptSha256 = snapshot.reviewReceiptSha256,
                    expectedSignature = snapshot.metadata.expectedSignature,
                    lastValidBlockHeight = snapshot.metadata.lastValidBlockHeight,
                )
            ) {
                StageFBAttemptGuard.BeginResult.STARTED_AND_PERSISTED -> Unit
                StageFBAttemptGuard.BeginResult.ALREADY_GUARDED ->
                    return "BLOCKED — ONE-ATTEMPT GUARD ALREADY EXISTS; DO NOT RETRY"
                StageFBAttemptGuard.BeginResult.INVALID_PUBLIC_METADATA ->
                    return "BLOCKED — ONE-ATTEMPT GUARD METADATA INVALID; NOTHING SUBMITTED"
                StageFBAttemptGuard.BeginResult.PERSISTENCE_FAILED ->
                    return "BLOCKED — ONE-ATTEMPT GUARD DID NOT PERSIST; NOTHING SUBMITTED"
            }

            val loaded = guard.load()
            if (loaded !is StageFBAttemptGuard.LoadResult.Present) {
                return "TERMINAL — ONE-ATTEMPT GUARD COULD NOT BE READ BACK; DO NOT SUBMIT OR RETRY"
            }

            val eligibility =
                StageFBPreArmingEligibility.evaluate(
                    metadata = snapshot.metadata,
                    claimedCandidateFingerprintSha256 = snapshot.candidateFingerprintSha256,
                    claimedReviewReceiptSha256 = snapshot.reviewReceiptSha256,
                    guardRecord = loaded.record,
                    currentBlockHeight = currentBlockHeight,
                )
            if (eligibility !is StageFBPreArmingEligibility.Result.Eligible) {
                return "TERMINAL — REVIEWED CANDIDATE IS NOT ELIGIBLE AFTER GUARD START; DO NOT RETRY"
            }

            val nativeResult = NativeBridge.submitStageFDevnetCandidateOnce(token)
            if (!nativeResult.startsWith("ok:")) {
                return "STAGE F-B SUBMISSION OUTCOME TERMINAL / POSSIBLY AMBIGUOUS — $nativeResult\nDO NOT RETRY. USE READ-ONLY RESOLUTION."
            }

            val returnedSignature = nativeResult.removePrefix("ok:")
            if (returnedSignature != eligibility.expectedSignature) {
                return "STAGE F-B TERMINAL — RETURNED SIGNATURE MISMATCH\nDO NOT RETRY. USE READ-ONLY RESOLUTION FOR THE PERSISTED EXPECTED SIGNATURE."
            }

            return "STAGE F-B ONE-SHOT DEVNET SUBMISSION ACCEPTED\nSignature: $returnedSignature\nNO RETRY / NO REBROADCAST. USE READ-ONLY RESOLUTION."
        } catch (error: Throwable) {
            return "STAGE F-B TERMINAL — ${error.javaClass.simpleName}\nDO NOT RETRY. USE READ-ONLY RESOLUTION IF THE GUARD WAS PERSISTED."
        } finally {
            try {
                NativeBridge.discardStageFDevnetCandidate(token)
            } catch (_: Throwable) {
                // Candidate is already consumed after any actual submission attempt.
            }
        }
    }

    private fun discardPreparedCandidate(): Boolean {
        val token =
            preparedCandidateToken
                ?: run {
                    preparedPublicReviewSnapshot = null
                    return true
                }

        return try {
            if (NativeBridge.discardStageFDevnetCandidate(token) == "ok") {
                preparedCandidateToken = null
                preparedPublicReviewSnapshot = null
                true
            } else {
                false
            }
        } catch (_: Throwable) {
            false
        }
    }

    private fun clearSensitiveFields() {
        passphraseField?.text?.clear()
        confirmationField?.text?.clear()
    }

    private companion object {
        const val STAGE_FB_PHYSICAL_SEND_ARMED = false
    }
}
