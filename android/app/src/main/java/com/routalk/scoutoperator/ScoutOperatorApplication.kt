package com.routalk.scoutoperator

import android.app.Activity
import android.app.AlertDialog
import android.app.Application
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.view.Gravity
import android.widget.Button
import android.widget.FrameLayout

internal class ScoutOperatorApplication : Application() {
    @Volatile
    private var updateCheckStarted = false

    override fun onCreate() {
        super.onCreate()

        registerActivityLifecycleCallbacks(
            object : ActivityLifecycleCallbacks {
                override fun onActivityResumed(activity: Activity) {
                    if (activity !is MainActivity) {
                        return
                    }

                    installManualUpdateControl(activity)

                    if (updateCheckStarted) {
                        return
                    }

                    updateCheckStarted = true

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

    private fun installManualUpdateControl(
        activity: Activity,
    ) {
        val contentRoot =
            activity.findViewById<FrameLayout>(
                android.R.id.content,
            )

        if (
            contentRoot.findViewWithTag<Button>(
                UPDATE_BUTTON_TAG,
            ) != null
        ) {
            return
        }

        val margin =
            (
                UPDATE_BUTTON_MARGIN_DP *
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
                            downloadUrl = downloadUrl,
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
        downloadUrl: String,
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

                openUpdate(
                    activity = activity,
                    downloadUrl = downloadUrl,
                )
            }
            .show()
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

    private fun openUpdate(
        activity: Activity,
        downloadUrl: String,
    ) {
        if (!isApprovedDownloadUrl(downloadUrl)) {
            return
        }

        val intent =
            Intent(
                Intent.ACTION_VIEW,
                Uri.parse(downloadUrl),
            ).apply {
                addCategory(Intent.CATEGORY_BROWSABLE)
            }

        try {
            activity.startActivity(intent)
        } catch (_: Throwable) {
            Unit
        }
    }

    private fun isApprovedDownloadUrl(
        value: String,
    ): Boolean =
        value.startsWith(
            APPROVED_DOWNLOAD_PREFIX,
        )

    private companion object {
        const val APPROVED_DOWNLOAD_PREFIX =
            "https://github.com/pjmcveyroutalk/scout-wallet-lab/"

        const val UPDATE_BUTTON_TAG =
            "scout-manual-update-control"

        const val UPDATE_BUTTON_MARGIN_DP = 12
    }
}
