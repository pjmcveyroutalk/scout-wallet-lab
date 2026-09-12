package com.routalk.scoutoperator

import android.app.Activity
import android.app.AlertDialog
import android.app.Application
import android.content.Intent
import android.os.Bundle
import android.view.Gravity
import android.widget.Button
import android.widget.FrameLayout
import android.widget.Toast

internal class ScoutOperatorApplication : Application() {
    override fun onCreate() {
        super.onCreate()

        registerActivityLifecycleCallbacks(
            object : ActivityLifecycleCallbacks {
                override fun onActivityResumed(activity: Activity) {
                    if (
                        activity !is MainActivity &&
                        activity !is CredentialRecoveryActivity
                    ) {
                        return
                    }

                    if (activity is MainActivity) {
                        installOperatorControls(activity)
                    }

                    checkForUpdate(
                        activity = activity,
                        showCurrentStatus = false,
                        triggerButton = null,
                    )
                }

                override fun onActivityCreated(
                    activity: Activity,
                    savedInstanceState: Bundle?,
                ) = Unit

                override fun onActivityStarted(activity: Activity) = Unit

                override fun onActivityPaused(activity: Activity) = Unit

                override fun onActivityStopped(activity: Activity) = Unit

                override fun onActivitySaveInstanceState(
                    activity: Activity,
                    outState: Bundle,
                ) = Unit

                override fun onActivityDestroyed(activity: Activity) = Unit
            },
        )
    }

    private fun installOperatorControls(
        activity: Activity,
    ) {
        val contentRoot =
            activity.findViewById<FrameLayout>(
                android.R.id.content,
            )

        installRecoveryControl(
            activity = activity,
            contentRoot = contentRoot,
        )

        installManualUpdateControl(
            activity = activity,
            contentRoot = contentRoot,
        )
    }

    private fun installRecoveryControl(
        activity: Activity,
        contentRoot: FrameLayout,
    ) {
        if (
            contentRoot.findViewWithTag<Button>(
                RECOVERY_BUTTON_TAG,
            ) != null
        ) {
            return
        }

        val margin =
            (
                CONTROL_MARGIN_DP *
                    activity.resources.displayMetrics.density
            ).toInt()

        val bottomOffset =
            (
                RECOVERY_BOTTOM_OFFSET_DP *
                    activity.resources.displayMetrics.density
            ).toInt()

        val button =
            Button(activity).apply {
                tag = RECOVERY_BUTTON_TAG
                text = "RECOVERY / RESTORE"
                contentDescription =
                    "Open Scout encrypted Devnet recovery and restore"

                setOnClickListener {
                    isEnabled = false

                    try {
                        activity.startActivity(
                            Intent(
                                activity,
                                LockedVaultRestoreActivity::class.java,
                            ),
                        )
                    } catch (_: Throwable) {
                        isEnabled = true
                    }
                }
            }

        val layoutParams =
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.WRAP_CONTENT,
                Gravity.END or Gravity.BOTTOM,
            ).apply {
                setMargins(
                    margin,
                    margin,
                    margin,
                    bottomOffset,
                )
            }

        contentRoot.addView(
            button,
            layoutParams,
        )
    }

    private fun installManualUpdateControl(
        activity: Activity,
        contentRoot: FrameLayout,
    ) {
        if (
            contentRoot.findViewWithTag<Button>(
                UPDATE_BUTTON_TAG,
            ) != null
        ) {
            return
        }

        val margin =
            (
                CONTROL_MARGIN_DP *
                    activity.resources.displayMetrics.density
            ).toInt()

        val button =
            Button(activity).apply {
                tag = UPDATE_BUTTON_TAG
                text = "CHECK UPDATE"
                contentDescription =
                    "Check for the newest Scout Devnet application update"

                setOnClickListener {
                    checkForUpdate(
                        activity = activity,
                        showCurrentStatus = true,
                        triggerButton = this,
                    )
                }
            }

        val layoutParams =
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.WRAP_CONTENT,
                Gravity.END or Gravity.BOTTOM,
            ).apply {
                setMargins(
                    margin,
                    margin,
                    margin,
                    margin,
                )
            }

        contentRoot.addView(
            button,
            layoutParams,
        )
    }

    private fun checkForUpdate(
        activity: Activity,
        showCurrentStatus: Boolean,
        triggerButton: Button?,
    ) {
        triggerButton?.isEnabled = false

        Thread {
            val result =
                try {
                    ScoutUpdateChecker.check(
                        activity.applicationContext,
                    )
                } catch (_: Throwable) {
                    null
                }

            val downloadUrl =
                result
                    ?.downloadUrl
                    ?.takeIf { it.isNotBlank() }

            activity.runOnUiThread {
                if (
                    activity.isFinishing ||
                    activity.isDestroyed
                ) {
                    return@runOnUiThread
                }

                triggerButton?.isEnabled = true

                when {
                    result == null -> {
                        if (showCurrentStatus) {
                            showStatusDialog(
                                activity = activity,
                                title = "Scout update check",
                                message =
                                    "UPDATE CHECK FAILED\n\n" +
                                        "Scout could not reach the " +
                                        "Devnet release service.",
                            )
                        }
                    }

                    !result.success -> {
                        if (showCurrentStatus) {
                            showStatusDialog(
                                activity = activity,
                                title = "Scout update check",
                                message = result.status,
                            )
                        }
                    }

                    result.updateAvailable &&
                        downloadUrl != null -> {
                        showUpdateDialog(
                            activity = activity,
                            result = result,
                        )
                    }

                    result.updateAvailable -> {
                        if (showCurrentStatus) {
                            showStatusDialog(
                                activity = activity,
                                title = "Scout update check",
                                message =
                                    "UPDATE FOUND, BUT DOWNLOAD " +
                                        "URL WAS NOT VERIFIED.",
                            )
                        }
                    }

                    showCurrentStatus -> {
                        showStatusDialog(
                            activity = activity,
                            title = "Scout is up to date",
                            message =
                                buildString {
                                    append("Installed: ")
                                    append(
                                        result.installedVersionName,
                                    )
                                    append("\n\n")
                                    append(
                                        "No newer Scout Devnet " +
                                            "build is available.",
                                    )
                                },
                        )
                    }
                }
            }
        }.start()
    }

    private fun showUpdateDialog(
        activity: Activity,
        result: ScoutUpdateChecker.UpdateResult,
    ) {
        val latestVersion =
            result.latestVersionName
                ?: "newer build"

        val shortSha =
            result.latestShortSha
                ?.takeIf { it.isNotBlank() }

        val releaseIdentity =
            if (shortSha == null) {
                latestVersion
            } else {
                "$latestVersion ($shortSha)"
            }

        AlertDialog.Builder(activity)
            .setTitle("Scout update available")
            .setMessage(
                buildString {
                    append("A newer Scout Devnet build is ready.")
                    append("\n\n")
                    append("Installed: ")
                    append(result.installedVersionName)
                    append("\n")
                    append("Available: ")
                    append(releaseIdentity)
                    append("\n\n")
                    append(
                        "Scout will download the APK, verify its " +
                            "published SHA-256 digest, and hand the " +
                            "verified file directly to Android's installer.",
                    )
                    append("\n\n")
                    append(
                        "Updating does not modify or replace " +
                            "your stored encrypted wallet vault.",
                    )
                },
            )
            .setCancelable(true)
            .setNegativeButton("LATER") { dialog, _ ->
                dialog.dismiss()
            }
            .setPositiveButton("UPDATE NOW") { dialog, _ ->
                dialog.dismiss()

                beginUpdate(
                    activity = activity,
                    result = result,
                )
            }
            .show()
    }

    private fun beginUpdate(
        activity: Activity,
        result: ScoutUpdateChecker.UpdateResult,
    ) {
        ScoutUpdateInstaller.begin(
            activity = activity,
            result = result,
        ) { status ->
            if (
                status.startsWith("UPDATE FAILED") ||
                status.startsWith("UPDATE BLOCKED")
            ) {
                showStatusDialog(
                    activity = activity,
                    title = "Scout update",
                    message = status,
                )
            } else {
                Toast.makeText(
                    activity,
                    status,
                    Toast.LENGTH_LONG,
                ).show()
            }
        }
    }

    private fun showStatusDialog(
        activity: Activity,
        title: String,
        message: String,
    ) {
        AlertDialog.Builder(activity)
            .setTitle(title)
            .setMessage(message)
            .setCancelable(true)
            .setPositiveButton("OK") { dialog, _ ->
                dialog.dismiss()
            }
            .show()
    }

    private companion object {
        const val UPDATE_BUTTON_TAG =
            "scout-manual-update-control"

        const val RECOVERY_BUTTON_TAG =
            "scout-recovery-restore-control"

        const val CONTROL_MARGIN_DP = 12

        const val RECOVERY_BOTTOM_OFFSET_DP = 76
    }
}
