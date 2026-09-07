package com.routalk.scoutoperator

import android.content.Context

internal object LockedVaultBackupManager {
    internal data class BackupResult(
        val success: Boolean,
        val publicAddress: String?,
        val backupJson: String?,
        val status: String,
    )

    fun create(
        context: Context,
        expectedAddress: String,
    ): BackupResult {
        if (expectedAddress.isBlank()) {
            return failure(
                "BACKUP BLOCKED — VERIFIED IDENTITY MISSING",
            )
        }

        val vaultStore =
            LockedVaultStore(context)

        if (!vaultStore.hasVault()) {
            return failure(
                "BACKUP BLOCKED — ENCRYPTED VAULT MISSING",
            )
        }

        val lockedVaultJson =
            vaultStore.loadVault()
                ?: return failure(
                    "BACKUP BLOCKED — ENCRYPTED VAULT UNREADABLE",
                )

        val identityResult =
            NativeBridge.lockedVaultDevnetAddress(
                lockedVaultJson,
            )

        if (!identityResult.startsWith("ok:")) {
            return failure(
                "BACKUP BLOCKED — IDENTITY NOT VERIFIED",
            )
        }

        val returnedAddress =
            identityResult.removePrefix("ok:")

        if (
            returnedAddress.isBlank() ||
            returnedAddress != expectedAddress
        ) {
            return failure(
                "BACKUP BLOCKED — IDENTITY MISMATCH",
            )
        }

        val backupResult =
            NativeBridge.createLockedVaultBackup(
                lockedVaultJson,
            )

        if (!backupResult.startsWith("ok:")) {
            return failure(
                "BACKUP FAILED — $backupResult",
            )
        }

        val backupJson =
            backupResult.removePrefix("ok:")

        if (backupJson.isBlank()) {
            return failure(
                "BACKUP FAILED — EMPTY BACKUP PACKAGE",
            )
        }

        val validationResult =
            NativeBridge.validateLockedVaultBackup(
                backupJson,
            )

        if (!validationResult.startsWith("ok:")) {
            return failure(
                "BACKUP FAILED — PACKAGE VALIDATION FAILED",
            )
        }

        val validatedAddress =
            validationResult.removePrefix("ok:")

        if (
            validatedAddress.isBlank() ||
            validatedAddress != expectedAddress
        ) {
            return failure(
                "BACKUP FAILED — VALIDATED IDENTITY MISMATCH",
            )
        }

        return BackupResult(
            success = true,
            publicAddress = validatedAddress,
            backupJson = backupJson,
            status = "VERIFIED — ENCRYPTED DEVNET BACKUP READY",
        )
    }

    fun validate(
        backupJson: String,
        expectedAddress: String,
    ): BackupResult {
        if (expectedAddress.isBlank()) {
            return failure(
                "BACKUP VALIDATION BLOCKED — VERIFIED IDENTITY MISSING",
            )
        }

        if (backupJson.isBlank()) {
            return failure(
                "BACKUP VALIDATION BLOCKED — BACKUP PACKAGE EMPTY",
            )
        }

        val validationResult =
            NativeBridge.validateLockedVaultBackup(
                backupJson,
            )

        if (!validationResult.startsWith("ok:")) {
            return failure(
                "BACKUP VALIDATION FAILED — $validationResult",
            )
        }

        val validatedAddress =
            validationResult.removePrefix("ok:")

        if (
            validatedAddress.isBlank() ||
            validatedAddress != expectedAddress
        ) {
            return failure(
                "BACKUP VALIDATION FAILED — IDENTITY MISMATCH",
            )
        }

        return BackupResult(
            success = true,
            publicAddress = validatedAddress,
            backupJson = backupJson,
            status = "VERIFIED — ENCRYPTED DEVNET BACKUP VALID",
        )
    }

    private fun failure(
        status: String,
    ): BackupResult =
        BackupResult(
            success = false,
            publicAddress = null,
            backupJson = null,
            status = status,
        )
}
