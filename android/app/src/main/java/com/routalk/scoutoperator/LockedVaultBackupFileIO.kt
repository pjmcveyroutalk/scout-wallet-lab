package com.routalk.scoutoperator

import android.content.ContentResolver
import android.content.Intent
import android.net.Uri

internal object LockedVaultBackupFileIO {
    const val MIME_TYPE = "application/json"

    private const val DEFAULT_FILE_NAME =
        "scout-devnet-locked-vault-backup.json"

    private const val MAX_BACKUP_BYTES =
        256 * 1024

    internal data class FileResult(
        val success: Boolean,
        val content: String?,
        val status: String,
    )

    fun createExportIntent(): Intent =
        Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = MIME_TYPE
            putExtra(
                Intent.EXTRA_TITLE,
                DEFAULT_FILE_NAME,
            )
        }

    fun createImportIntent(): Intent =
        Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = MIME_TYPE
        }

    fun writeBackup(
        contentResolver: ContentResolver,
        destination: Uri,
        backupJson: String,
    ): FileResult {
        if (backupJson.isBlank()) {
            return failure(
                "BACKUP EXPORT BLOCKED — BACKUP PACKAGE EMPTY",
            )
        }

        val encoded =
            backupJson.toByteArray(Charsets.UTF_8)

        if (encoded.size > MAX_BACKUP_BYTES) {
            return failure(
                "BACKUP EXPORT BLOCKED — BACKUP PACKAGE TOO LARGE",
            )
        }

        return try {
            val output =
                contentResolver.openOutputStream(
                    destination,
                    "wt",
                )
                    ?: return failure(
                        "BACKUP EXPORT FAILED — DESTINATION UNAVAILABLE",
                    )

            output.use { stream ->
                stream.write(encoded)
                stream.flush()
            }

            FileResult(
                success = true,
                content = null,
                status =
                    "VERIFIED — ENCRYPTED DEVNET BACKUP EXPORTED",
            )
        } catch (error: Throwable) {
            failure(
                "BACKUP EXPORT FAILED — ${error.javaClass.simpleName}",
            )
        } finally {
            encoded.fill(0)
        }
    }

    fun readBackup(
        contentResolver: ContentResolver,
        source: Uri,
    ): FileResult {
        return try {
            val input =
                contentResolver.openInputStream(source)
                    ?: return failure(
                        "BACKUP IMPORT FAILED — SOURCE UNAVAILABLE",
                    )

            val bytes =
                input.use { stream ->
                    val buffer =
                        ByteArray(MAX_BACKUP_BYTES + 1)

                    var totalRead = 0

                    while (totalRead < buffer.size) {
                        val count =
                            stream.read(
                                buffer,
                                totalRead,
                                buffer.size - totalRead,
                            )

                        if (count < 0) {
                            break
                        }

                        if (count == 0) {
                            continue
                        }

                        totalRead += count
                    }

                    if (totalRead > MAX_BACKUP_BYTES) {
                        buffer.fill(0)

                        return failure(
                            "BACKUP IMPORT BLOCKED — FILE TOO LARGE",
                        )
                    }

                    buffer.copyOf(totalRead)
                        .also {
                            buffer.fill(0)
                        }
                }

            val backupJson =
                try {
                    bytes.toString(Charsets.UTF_8)
                } finally {
                    bytes.fill(0)
                }

            if (backupJson.isBlank()) {
                return failure(
                    "BACKUP IMPORT BLOCKED — FILE EMPTY",
                )
            }

            FileResult(
                success = true,
                content = backupJson,
                status =
                    "ENCRYPTED BACKUP FILE LOADED — VALIDATION REQUIRED",
            )
        } catch (error: Throwable) {
            failure(
                "BACKUP IMPORT FAILED — ${error.javaClass.simpleName}",
            )
        }
    }

    private fun failure(
        status: String,
    ): FileResult =
        FileResult(
            success = false,
            content = null,
            status = status,
        )
}
