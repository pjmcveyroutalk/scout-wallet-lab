package com.routalk.scoutoperator

import android.app.Activity
import android.app.DownloadManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.Settings
import java.io.File
import java.security.MessageDigest

internal object ScoutUpdateInstaller {
    private const val APK_MIME_TYPE =
        "application/vnd.android.package-archive"

    private const val APPROVED_DOWNLOAD_PREFIX =
        "https://github.com/pjmcveyroutalk/scout-wallet-lab/"

    private const val DOWNLOAD_TIMEOUT_MS = 15L * 60L * 1000L
    private const val POLL_INTERVAL_MS = 500L

    @Volatile
    private var activeDownloadId: Long? = null

    fun begin(
        activity: Activity,
        result: ScoutUpdateChecker.UpdateResult,
        onStatus: (String) -> Unit,
    ) {
        val downloadUrl =
            result.downloadUrl
                ?.takeIf { value ->
                    value.startsWith(APPROVED_DOWNLOAD_PREFIX)
                }

        val assetName =
            result.assetName
                ?.takeIf { value ->
                    value.endsWith(".apk") &&
                        !value.contains('/') &&
                        !value.contains('\\')
                }

        val expectedSha256 =
            result.expectedSha256
                ?.takeIf { value ->
                    value.matches(Regex("^[0-9a-f]{64}$"))
                }

        if (
            downloadUrl == null ||
            assetName == null ||
            expectedSha256 == null
        ) {
            onStatus(
                "UPDATE BLOCKED — RELEASE IDENTITY WAS NOT VERIFIED",
            )
            return
        }

        if (
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.O &&
            !activity.packageManager.canRequestPackageInstalls()
        ) {
            onStatus(
                "ONE-TIME SETUP — ALLOW SCOUT TO INSTALL APP UPDATES, " +
                    "THEN RETURN AND TAP CHECK UPDATE AGAIN.",
            )

            val settingsIntent =
                Intent(
                    Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                    Uri.parse("package:${activity.packageName}"),
                )

            try {
                activity.startActivity(settingsIntent)
            } catch (_: Throwable) {
                Unit
            }

            return
        }

        if (activeDownloadId != null) {
            onStatus("UPDATE DOWNLOAD ALREADY IN PROGRESS")
            return
        }

        val downloadManager =
            activity.getSystemService(DownloadManager::class.java)

        if (downloadManager == null) {
            onStatus("UPDATE FAILED — DOWNLOAD SERVICE UNAVAILABLE")
            return
        }

        val destinationDirectory =
            activity.getExternalFilesDir(
                Environment.DIRECTORY_DOWNLOADS,
            )

        if (destinationDirectory == null) {
            onStatus("UPDATE FAILED — DOWNLOAD STORAGE UNAVAILABLE")
            return
        }

        val destinationFile =
            File(
                destinationDirectory,
                assetName,
            )

        if (
            destinationFile.exists() &&
            !destinationFile.delete()
        ) {
            onStatus("UPDATE FAILED — OLD DOWNLOAD COULD NOT BE CLEARED")
            return
        }

        val request =
            DownloadManager.Request(
                Uri.parse(downloadUrl),
            ).apply {
                setTitle(
                    "Scout ${result.latestVersionName ?: "Devnet update"}",
                )
                setDescription("Downloading verified Scout update")
                setMimeType(APK_MIME_TYPE)
                setAllowedOverMetered(true)
                setAllowedOverRoaming(false)
                setNotificationVisibility(
                    DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED,
                )
                setDestinationInExternalFilesDir(
                    activity,
                    Environment.DIRECTORY_DOWNLOADS,
                    assetName,
                )
            }

        val downloadId =
            try {
                downloadManager.enqueue(request)
            } catch (error: Throwable) {
                onStatus(
                    "UPDATE FAILED — DOWNLOAD START ${error.javaClass.simpleName}",
                )
                return
            }

        activeDownloadId = downloadId
        onStatus("DOWNLOADING VERIFIED SCOUT UPDATE…")

        Thread {
            monitorDownload(
                activity = activity,
                downloadManager = downloadManager,
                downloadId = downloadId,
                expectedSha256 = expectedSha256,
                onStatus = onStatus,
            )
        }.start()
    }

    private fun monitorDownload(
        activity: Activity,
        downloadManager: DownloadManager,
        downloadId: Long,
        expectedSha256: String,
        onStatus: (String) -> Unit,
    ) {
        val startedAt =
            System.currentTimeMillis()

        try {
            while (
                System.currentTimeMillis() - startedAt <
                DOWNLOAD_TIMEOUT_MS
            ) {
                val query =
                    DownloadManager.Query()
                        .setFilterById(downloadId)

                val snapshot =
                    try {
                        downloadManager.query(query)
                    } catch (error: Throwable) {
                        postStatus(
                            activity,
                            onStatus,
                            "UPDATE FAILED — DOWNLOAD QUERY " +
                                error.javaClass.simpleName,
                        )
                        return
                    }

                snapshot.use { cursor ->
                    if (!cursor.moveToFirst()) {
                        postStatus(
                            activity,
                            onStatus,
                            "UPDATE FAILED — DOWNLOAD DISAPPEARED",
                        )
                        return
                    }

                    val status =
                        cursor.getInt(
                            cursor.getColumnIndexOrThrow(
                                DownloadManager.COLUMN_STATUS,
                            ),
                        )

                    when (status) {
                        DownloadManager.STATUS_SUCCESSFUL -> {
                            val downloadedUri =
                                downloadManager.getUriForDownloadedFile(
                                    downloadId,
                                )

                            if (downloadedUri == null) {
                                postStatus(
                                    activity,
                                    onStatus,
                                    "UPDATE FAILED — DOWNLOADED APK UNAVAILABLE",
                                )
                                return
                            }

                            val digestMatches =
                                verifySha256(
                                    context = activity.applicationContext,
                                    uri = downloadedUri,
                                    expectedSha256 = expectedSha256,
                                )

                            if (!digestMatches) {
                                downloadManager.remove(downloadId)
                                postStatus(
                                    activity,
                                    onStatus,
                                    "UPDATE BLOCKED — APK DIGEST MISMATCH",
                                )
                                return
                            }

                            postStatus(
                                activity,
                                onStatus,
                                "DOWNLOAD VERIFIED — OPENING ANDROID INSTALLER…",
                            )

                            launchInstaller(
                                context = activity.applicationContext,
                                uri = downloadedUri,
                                onFailure = { message ->
                                    postStatus(
                                        activity,
                                        onStatus,
                                        message,
                                    )
                                },
                            )
                            return
                        }

                        DownloadManager.STATUS_FAILED -> {
                            val reason =
                                cursor.getInt(
                                    cursor.getColumnIndexOrThrow(
                                        DownloadManager.COLUMN_REASON,
                                    ),
                                )

                            postStatus(
                                activity,
                                onStatus,
                                "UPDATE FAILED — DOWNLOAD REASON $reason",
                            )
                            return
                        }
                    }
                }

                try {
                    Thread.sleep(POLL_INTERVAL_MS)
                } catch (_: InterruptedException) {
                    Thread.currentThread().interrupt()
                    postStatus(
                        activity,
                        onStatus,
                        "UPDATE FAILED — DOWNLOAD INTERRUPTED",
                    )
                    return
                }
            }

            postStatus(
                activity,
                onStatus,
                "UPDATE FAILED — DOWNLOAD TIMED OUT",
            )
        } finally {
            activeDownloadId = null
        }
    }

    private fun verifySha256(
        context: Context,
        uri: Uri,
        expectedSha256: String,
    ): Boolean {
        val digest =
            MessageDigest.getInstance("SHA-256")

        return try {
            context.contentResolver
                .openInputStream(uri)
                ?.use { input ->
                    val buffer =
                        ByteArray(DEFAULT_BUFFER_SIZE)

                    while (true) {
                        val count =
                            input.read(buffer)

                        if (count < 0) {
                            break
                        }

                        if (count > 0) {
                            digest.update(
                                buffer,
                                0,
                                count,
                            )
                        }
                    }
                }
                ?: return false

            val actual =
                digest.digest()
                    .joinToString(separator = "") { byte ->
                        "%02x".format(byte)
                    }

            MessageDigest.isEqual(
                actual.toByteArray(Charsets.US_ASCII),
                expectedSha256.toByteArray(Charsets.US_ASCII),
            )
        } catch (_: Throwable) {
            false
        }
    }

    private fun launchInstaller(
        context: Context,
        uri: Uri,
        onFailure: (String) -> Unit,
    ) {
        val intent =
            Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(
                    uri,
                    APK_MIME_TYPE,
                )
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }

        try {
            context.startActivity(intent)
        } catch (error: Throwable) {
            onFailure(
                "UPDATE FAILED — INSTALLER ${error.javaClass.simpleName}",
            )
        }
    }

    private fun postStatus(
        activity: Activity,
        onStatus: (String) -> Unit,
        message: String,
    ) {
        activity.runOnUiThread {
            if (
                !activity.isFinishing &&
                !activity.isDestroyed
            ) {
                onStatus(message)
            }
        }
    }
}
