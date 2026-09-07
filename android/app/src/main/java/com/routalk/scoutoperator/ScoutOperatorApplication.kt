package com.routalk.scoutoperator

import android.app.Activity
import android.app.AlertDialog
import android.app.Application
import android.content.Intent
import android.net.Uri
import android.os.Bundle

internal class ScoutOperatorApplication : Application() {
    @Volatile
    private var updateCheckStarted = false

    override fun onCreate() {
        super.onCreate()

        registerActivityLifecycleCallbacks(
            object : ActivityLifecycleCallbacks {
                override fun onActivityResumed(activity: Activity) {
                    if (
                        activity !is MainActivity ||
                        updateCheckStarted
                    ) {
                        return
                    }

                    updateCheckStarted = true
                    checkForUpdate(activity)
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

    private fun checkForUpdate(
        activity: Activity,
    ) {
        Thread {
            val result =
                try {
                    ScoutUpdateChecker.check(
                        activity.applicationContext,
                    )
                } catch (_: Throwable) {
                    null
                }

            if (
                result == null ||
                !result.success ||
                !result.updateAvailable
            ) {
                return@Thread
            }

            val downloadUrl =
                result.downloadUrl
                    ?.takeIf { it.isNotBlank() }
                    ?: return@Thread

            activity.runOnUiThread {
                if (
                    activity.isFinishing ||
                    activity.isDestroyed
                ) {
                    return@runOnUiThread
                }

                showUpdateDialog(
                    activity = activity,
                    result = result,
                    downloadUrl = downloadUrl,
                )
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
    }
}
