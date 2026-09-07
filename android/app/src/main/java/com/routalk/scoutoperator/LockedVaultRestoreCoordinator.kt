package com.routalk.scoutoperator

import android.content.Context

internal object LockedVaultRestoreCoordinator {
    internal data class RestoreResult(
        val success: Boolean,
        val publicAddress: String?,
        val status: String,
    )

    fun restoreIntoEmptyStorage(
        context: Context,
        backupJson: String,
    ): RestoreResult {
        if (backupJson.isBlank()) {
            return failure(
                "RESTORE BLOCKED — BACKUP PACKAGE EMPTY",
            )
        }

        val vaultStore =
            LockedVaultStore(context)

        if (vaultStore.hasVault()) {
            return failure(
                "RESTORE BLOCKED — EXISTING ENCRYPTED VAULT PRESENT",
            )
        }

        val validationResult =
            NativeBridge.validateLockedVaultBackup(
                backupJson,
            )

        if (!validationResult.startsWith("ok:")) {
            return failure(
                "RESTORE BLOCKED — BACKUP VALIDATION FAILED",
            )
        }

        val validatedAddress =
            validationResult
                .removePrefix("ok:")
                .trim()

        if (validatedAddress.isBlank()) {
            return failure(
                "RESTORE BLOCKED — VALIDATED IDENTITY MISSING",
            )
        }

        val extractionResult =
            NativeBridge.extractValidatedLockedVaultBackup(
                backupJson,
            )

        if (!extractionResult.startsWith("ok:")) {
            return failure(
                "RESTORE BLOCKED — ENCRYPTED VAULT EXTRACTION FAILED",
            )
        }

        val extractionPayload =
            extractionResult.removePrefix("ok:")

        val separatorIndex =
            extractionPayload.indexOf(':')

        if (
            separatorIndex <= 0 ||
            separatorIndex >= extractionPayload.lastIndex
        ) {
            return failure(
                "RESTORE BLOCKED — EXTRACTION RESPONSE INVALID",
            )
        }

        val extractedAddress =
            extractionPayload
                .substring(
                    startIndex = 0,
                    endIndex = separatorIndex,
                )
                .trim()

        val extractedVaultJson =
            extractionPayload
                .substring(separatorIndex + 1)
                .trim()

        if (extractedAddress.isBlank()) {
            return failure(
                "RESTORE BLOCKED — EXTRACTED IDENTITY MISSING",
            )
        }

        if (extractedVaultJson.isBlank()) {
            return failure(
                "RESTORE BLOCKED — EXTRACTED ENCRYPTED VAULT EMPTY",
            )
        }

        if (extractedAddress != validatedAddress) {
            return failure(
                "RESTORE BLOCKED — BACKUP IDENTITY MISMATCH",
            )
        }

        val extractedIdentityResult =
            NativeBridge.lockedVaultDevnetAddress(
                extractedVaultJson,
            )

        if (!extractedIdentityResult.startsWith("ok:")) {
            return failure(
                "RESTORE BLOCKED — EXTRACTED VAULT IDENTITY INVALID",
            )
        }

        val extractedVaultAddress =
            extractedIdentityResult
                .removePrefix("ok:")
                .trim()

        if (
            extractedVaultAddress.isBlank() ||
            extractedVaultAddress != validatedAddress
        ) {
            return failure(
                "RESTORE BLOCKED — EXTRACTED VAULT IDENTITY MISMATCH",
            )
        }

        if (vaultStore.hasVault()) {
            return failure(
                "RESTORE BLOCKED — STORAGE BECAME OCCUPIED",
            )
        }

        if (!vaultStore.saveVault(extractedVaultJson)) {
            return failure(
                "RESTORE FAILED — ENCRYPTED VAULT WRITE FAILED",
            )
        }

        val reloadedVaultJson =
            vaultStore.loadVault()

        if (reloadedVaultJson == null) {
            return rollbackFailure(
                vaultStore = vaultStore,
                status =
                    "RESTORE FAILED — WRITTEN VAULT COULD NOT BE RELOADED",
            )
        }

        if (reloadedVaultJson != extractedVaultJson) {
            return rollbackFailure(
                vaultStore = vaultStore,
                status =
                    "RESTORE FAILED — WRITTEN VAULT DID NOT MATCH BACKUP",
            )
        }

        val restoredIdentityResult =
            NativeBridge.lockedVaultDevnetAddress(
                reloadedVaultJson,
            )

        if (!restoredIdentityResult.startsWith("ok:")) {
            return rollbackFailure(
                vaultStore = vaultStore,
                status =
                    "RESTORE FAILED — RESTORED VAULT IDENTITY INVALID",
            )
        }

        val restoredAddress =
            restoredIdentityResult
                .removePrefix("ok:")
                .trim()

        if (
            restoredAddress.isBlank() ||
            restoredAddress != validatedAddress
        ) {
            return rollbackFailure(
                vaultStore = vaultStore,
                status =
                    "RESTORE FAILED — RESTORED IDENTITY MISMATCH",
            )
        }

        return RestoreResult(
            success = true,
            publicAddress = restoredAddress,
            status =
                "RESTORE VERIFIED — ENCRYPTED DEVNET VAULT RESTORED",
        )
    }

    private fun rollbackFailure(
        vaultStore: LockedVaultStore,
        status: String,
    ): RestoreResult {
        val cleared =
            vaultStore.clearVault()

        return if (cleared) {
            failure(
                "$status — NEW RESTORE WRITE REMOVED",
            )
        } else {
            failure(
                "$status — CRITICAL: RESTORE CLEANUP FAILED",
            )
        }
    }

    private fun failure(
        status: String,
    ): RestoreResult =
        RestoreResult(
            success = false,
            publicAddress = null,
            status = status,
        )
}
