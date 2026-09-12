package com.routalk.scoutoperator

import android.app.Activity
import android.os.Bundle
import android.view.Gravity
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView

class StageFBGateActivity : Activity() {
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

        root.addView(text("SCOUT STAGE F-B", 24f))
        root.addView(text("DEVNET ONLY", 18f))
        root.addView(text("MAINNET — DISABLED", 14f))
        root.addView(text("ARBITRARY SIGNING — DISABLED", 14f))
        root.addView(
            text(
                "Stage F-B is the separate operator gate that follows the completed Stage F-A presubmit proof. The ledger-write action is not armed in this build.",
                14f,
            ),
        )
        root.addView(
            text(
                "This screen can now perform a manual read-only Devnet resolution check for the exact public signature already stored in the one-attempt guard. It cannot create, sign, replay, replace, or submit a transaction.",
                14f,
            ),
        )

        val status =
            text(
                "IMPLEMENTATION GATE — NOT ARMED",
                17f,
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

        val guardStatus =
            text(
                describeAttemptGuardState(),
                14f,
            )
        root.addView(guardStatus)

        val refresh =
            Button(this).apply {
                text = "REFRESH READ-ONLY RESOLUTION"
                contentDescription = "Refresh the public Stage F-B Devnet resolution state"
                isEnabled = StageFBAttemptGuard(this@StageFBGateActivity).load() is
                    StageFBAttemptGuard.LoadResult.Present
            }
        fullWidth(refresh)
        root.addView(refresh)

        refresh.setOnClickListener {
            refresh.isEnabled = false
            status.text = "READ-ONLY RESOLUTION — CHECKING DEVNET"

            Thread {
                val outcome = refreshResolution()

                runOnUiThread {
                    guardStatus.text = describeAttemptGuardState()
                    status.text = outcome
                    refresh.isEnabled =
                        StageFBAttemptGuard(this).load() is StageFBAttemptGuard.LoadResult.Present
                }
            }.start()
        }

        val close =
            Button(this).apply {
                text = "CLOSE"
                contentDescription = "Close the Scout Stage F-B gate"
                setOnClickListener { finish() }
            }
        fullWidth(close)
        root.addView(close)

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

    private fun refreshResolution(): String {
        val guard = StageFBAttemptGuard(this)
        val loaded = guard.load()
        if (loaded !is StageFBAttemptGuard.LoadResult.Present) {
            return "READ-ONLY RESOLUTION BLOCKED — GUARD NOT AVAILABLE"
        }

        val blockHeightResult = NativeBridge.devnetBlockHeight()
        if (!blockHeightResult.startsWith("ok:")) {
            return "READ-ONLY RESOLUTION FAILED — BLOCK HEIGHT UNAVAILABLE"
        }

        val currentBlockHeight = blockHeightResult.removePrefix("ok:").toLongOrNull()
        if (currentBlockHeight == null || currentBlockHeight <= 0L) {
            return "READ-ONLY RESOLUTION FAILED — INVALID BLOCK HEIGHT"
        }

        val observedStatus =
            when (val result = StageFBReadOnlyStatusClient.fetch(loaded.record.expectedSignature)) {
                StageFBReadOnlyStatusClient.Result.Missing -> null
                is StageFBReadOnlyStatusClient.Result.Observed ->
                    StageFBReadOnlyResolution.ObservedStatus(
                        slot = result.slot,
                        confirmationState = result.confirmationState,
                        hasExecutionError = result.hasExecutionError,
                    )
                StageFBReadOnlyStatusClient.Result.Invalid ->
                    return "READ-ONLY RESOLUTION FAILED — DEVNET STATUS INVALID"
            }

        val resolution =
            StageFBReadOnlyResolution.resolve(
                observedStatus = observedStatus,
                currentBlockHeight = currentBlockHeight,
                lastValidBlockHeight = loaded.record.lastValidBlockHeight,
            )

        if (resolution == StageFBReadOnlyResolution.Resolution.Invalid) {
            return "READ-ONLY RESOLUTION FAILED — FAIL-CLOSED RESULT"
        }

        return when (
            guard.updateFromResolution(
                candidateFingerprintSha256 = loaded.record.candidateFingerprintSha256,
                reviewReceiptSha256 = loaded.record.reviewReceiptSha256,
                expectedSignature = loaded.record.expectedSignature,
                resolution = resolution,
            )
        ) {
            StageFBAttemptGuard.UpdateResult.UPDATED_AND_PERSISTED ->
                "READ-ONLY RESOLUTION — ${resolutionLabel(resolution)}"
            StageFBAttemptGuard.UpdateResult.NO_GUARD_PRESENT ->
                "READ-ONLY RESOLUTION FAILED — GUARD DISAPPEARED"
            StageFBAttemptGuard.UpdateResult.CORRUPT_GUARD ->
                "READ-ONLY RESOLUTION FAILED — GUARD CORRUPT / FAIL CLOSED"
            StageFBAttemptGuard.UpdateResult.INVALID_TRANSITION ->
                "READ-ONLY RESOLUTION FAILED — INVALID STATE TRANSITION"
            StageFBAttemptGuard.UpdateResult.PERSISTENCE_FAILED ->
                "READ-ONLY RESOLUTION FAILED — STATUS PERSISTENCE FAILED"
        }
    }

    private fun resolutionLabel(resolution: StageFBReadOnlyResolution.Resolution): String =
        when (resolution) {
            StageFBReadOnlyResolution.Resolution.Pending -> "PENDING"
            is StageFBReadOnlyResolution.Resolution.Processed -> "PROCESSED"
            is StageFBReadOnlyResolution.Resolution.Confirmed -> "CONFIRMED"
            is StageFBReadOnlyResolution.Resolution.Finalized -> "FINALIZED"
            is StageFBReadOnlyResolution.Resolution.Failed -> "FAILED"
            StageFBReadOnlyResolution.Resolution.Expired -> "EXPIRED"
            StageFBReadOnlyResolution.Resolution.Invalid -> "INVALID"
        }

    private fun describeAttemptGuardState(): String =
        when (val loaded = StageFBAttemptGuard(this).load()) {
            StageFBAttemptGuard.LoadResult.Empty ->
                "ONE-ATTEMPT GUARD — CLEAR\nNo Stage F-B ledger attempt is recorded on this device."

            StageFBAttemptGuard.LoadResult.Corrupt ->
                "ONE-ATTEMPT GUARD — CORRUPT / FAIL CLOSED\nStage F-B must remain read-only until the guard state is repaired deliberately."

            is StageFBAttemptGuard.LoadResult.Present ->
                buildString {
                    append("ONE-ATTEMPT GUARD — RECORD PRESENT")
                    append("\nPublic status: ")
                    append(loaded.record.publicStatus.name)
                    append("\nLast valid block height: ")
                    append(loaded.record.lastValidBlockHeight)
                    append("\nCandidate fingerprint SHA-256: ")
                    append(loaded.record.candidateFingerprintSha256)
                    append("\nReview receipt SHA-256: ")
                    append(loaded.record.reviewReceiptSha256)
                    append("\nBINDING OBSERVATION ONLY — EXECUTION NOT AUTHORIZED")
                    append("\nNo replay or second attempt is available from this screen.")
                }
        }
}
