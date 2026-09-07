package com.routalk.scoutoperator

import android.content.Context
import android.os.Build
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

internal object ScoutUpdateChecker {
    private const val RELEASE_API_URL =
        "https://api.github.com/repos/pjmcveyroutalk/scout-wallet-lab/releases/tags/devnet-latest"

    private const val CONNECT_TIMEOUT_MS = 10_000
    private const val READ_TIMEOUT_MS = 10_000

    private val apkNamePattern =
        Regex(
            pattern =
                """^scout-operator-v0\.2\.(\d+)-([0-9a-fA-F]{7})\.apk$""",
        )

    internal data class UpdateResult(
        val success: Boolean,
        val updateAvailable: Boolean,
        val installedVersionCode: Long,
        val installedVersionName: String,
        val latestVersionCode: Long?,
        val latestVersionName: String?,
        val latestShortSha: String?,
        val downloadUrl: String?,
        val assetName: String?,
        val status: String,
    )

    fun check(
        context: Context,
    ): UpdateResult {
        val installed =
            installedVersion(context)

        if (installed == null) {
            return failure(
                installedVersionCode = 0L,
                installedVersionName = "unknown",
                status = "UPDATE CHECK FAILED — INSTALLED VERSION UNAVAILABLE",
            )
        }

        val connection =
            try {
                URL(RELEASE_API_URL)
                    .openConnection() as HttpURLConnection
            } catch (error: Throwable) {
                return failure(
                    installedVersionCode = installed.versionCode,
                    installedVersionName = installed.versionName,
                    status =
                        "UPDATE CHECK FAILED — CONNECTION SETUP " +
                            error.javaClass.simpleName,
                )
            }

        return try {
            configureConnection(connection)

            val responseCode =
                connection.responseCode

            if (responseCode !in 200..299) {
                return failure(
                    installedVersionCode = installed.versionCode,
                    installedVersionName = installed.versionName,
                    status =
                        "UPDATE CHECK FAILED — GITHUB HTTP $responseCode",
                )
            }

            val responseBody =
                connection.inputStream
                    .bufferedReader(Charsets.UTF_8)
                    .use { reader ->
                        reader.readText()
                    }

            if (responseBody.isBlank()) {
                return failure(
                    installedVersionCode = installed.versionCode,
                    installedVersionName = installed.versionName,
                    status = "UPDATE CHECK FAILED — EMPTY RELEASE RESPONSE",
                )
            }

            val latest =
                parseLatestApk(responseBody)
                    ?: return failure(
                        installedVersionCode = installed.versionCode,
                        installedVersionName = installed.versionName,
                        status = "UPDATE CHECK FAILED — NO VALID DEVNET APK FOUND",
                    )

            val updateAvailable =
                latest.versionCode > installed.versionCode

            UpdateResult(
                success = true,
                updateAvailable = updateAvailable,
                installedVersionCode = installed.versionCode,
                installedVersionName = installed.versionName,
                latestVersionCode = latest.versionCode,
                latestVersionName = latest.versionName,
                latestShortSha = latest.shortSha,
                downloadUrl = latest.downloadUrl,
                assetName = latest.assetName,
                status =
                    if (updateAvailable) {
                        "UPDATE AVAILABLE — ${latest.versionName}"
                    } else {
                        "UP TO DATE — ${installed.versionName}"
                    },
            )
        } catch (error: Throwable) {
            failure(
                installedVersionCode = installed.versionCode,
                installedVersionName = installed.versionName,
                status =
                    "UPDATE CHECK FAILED — ${error.javaClass.simpleName}",
            )
        } finally {
            connection.disconnect()
        }
    }

    private fun configureConnection(
        connection: HttpURLConnection,
    ) {
        connection.requestMethod = "GET"
        connection.connectTimeout = CONNECT_TIMEOUT_MS
        connection.readTimeout = READ_TIMEOUT_MS
        connection.instanceFollowRedirects = true
        connection.useCaches = false

        connection.setRequestProperty(
            "Accept",
            "application/vnd.github+json",
        )
        connection.setRequestProperty(
            "User-Agent",
            "Scout-Operator-Devnet",
        )
        connection.setRequestProperty(
            "X-GitHub-Api-Version",
            "2022-11-28",
        )
    }

    private fun parseLatestApk(
        responseBody: String,
    ): ReleaseApk? {
        val release =
            JSONObject(responseBody)

        val assets =
            release.optJSONArray("assets")
                ?: return null

        var latest: ReleaseApk? = null

        for (index in 0 until assets.length()) {
            val asset =
                assets.optJSONObject(index)
                    ?: continue

            val name =
                asset.optString("name")
                    .takeIf { it.isNotBlank() }
                    ?: continue

            val match =
                apkNamePattern.matchEntire(name)
                    ?: continue

            val versionCode =
                match.groupValues[1]
                    .toLongOrNull()
                    ?: continue

            val shortSha =
                match.groupValues[2]
                    .lowercase()

            val downloadUrl =
                asset.optString(
                    "browser_download_url",
                )
                    .takeIf {
                        it.startsWith(
                            "https://github.com/pjmcveyroutalk/scout-wallet-lab/",
                        )
                    }
                    ?: continue

            val candidate =
                ReleaseApk(
                    versionCode = versionCode,
                    versionName = "0.2.$versionCode",
                    shortSha = shortSha,
                    downloadUrl = downloadUrl,
                    assetName = name,
                )

            val currentLatest =
                latest

            if (
                currentLatest == null ||
                candidate.versionCode > currentLatest.versionCode
            ) {
                latest = candidate
            }
        }

        return latest
    }

    private fun installedVersion(
        context: Context,
    ): InstalledVersion? {
        return try {
            val packageInfo =
                context.packageManager.getPackageInfo(
                    context.packageName,
                    0,
                )

            val versionCode =
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                    packageInfo.longVersionCode
                } else {
                    @Suppress("DEPRECATION")
                    packageInfo.versionCode.toLong()
                }

            val versionName =
                packageInfo.versionName
                    ?.takeIf { it.isNotBlank() }
                    ?: "unknown"

            InstalledVersion(
                versionCode = versionCode,
                versionName = versionName,
            )
        } catch (_: Throwable) {
            null
        }
    }

    private fun failure(
        installedVersionCode: Long,
        installedVersionName: String,
        status: String,
    ): UpdateResult =
        UpdateResult(
            success = false,
            updateAvailable = false,
            installedVersionCode = installedVersionCode,
            installedVersionName = installedVersionName,
            latestVersionCode = null,
            latestVersionName = null,
            latestShortSha = null,
            downloadUrl = null,
            assetName = null,
            status = status,
        )

    private data class InstalledVersion(
        val versionCode: Long,
        val versionName: String,
    )

    private data class ReleaseApk(
        val versionCode: Long,
        val versionName: String,
        val shortSha: String,
        val downloadUrl: String,
        val assetName: String,
    )
}
