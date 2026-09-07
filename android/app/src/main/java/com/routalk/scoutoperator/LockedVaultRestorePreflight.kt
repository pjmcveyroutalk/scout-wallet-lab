package com.routalk.scoutoperator

import android.content.Context

internal object LockedVaultRestorePreflight {
    internal data class PreflightResult(
        val success: Boolean,
        val publicAddress: String?,
        val status: String,
    )

    fun verify(
        context: Context,
        backupJson: String,
        expectedAddress: String,
    ): PreflightResult {
        if (expectedAddress.isBlank()) {
            return failure(
                "RESTORE PREFLIGHT BLOCKED — VERIFIED IDENTITY MISSING",
            )
        }

        if (backupJson.isBlank()) {
            return failure(
                "RESTORE PREFLIGHT BLOCKED — BACKUP PACKAGE EMPTY",
            )
        }

        val vaultStore =
            try {
                LockedVaultStore(context)
            } catch (error: Throwable) {
                return failure(
                    "RESTORE PREFLIGHT BLOCKED — STORAGE UNAVAILABLE:" +
                        error.javaClass.simpleName,
                )
            }

        if (!vaultStore.hasVault()) {
            return failure(
                "RESTORE PREFLIGHT BLOCKED — ACTIVE ENCRYPTED VAULT MISSING",
            )
        }

        val activeVaultBefore =
            vaultStore.loadVault()
                ?: return failure(
                    "RESTORE PREFLIGHT BLOCKED — ACTIVE ENCRYPTED VAULT UNREADABLE",
                )

        val activeIdentityResult =
            try {
                NativeBridge.lockedVaultDevnetAddress(
                    activeVaultBefore,
                )
            } catch (error: Throwable) {
                return failure(
                    "RESTORE PREFLIGHT BLOCKED — ACTIVE IDENTITY CHECK FAILED:" +
                        error.javaClass.simpleName,
                )
            }

        if (!activeIdentityResult.startsWith("ok:")) {
            return failure(
                "RESTORE PREFLIGHT BLOCKED — ACTIVE IDENTITY NOT VERIFIED",
            )
        }

        val activeAddress =
            activeIdentityResult.removePrefix("ok:")

        if (
            activeAddress.isBlank() ||
            activeAddress != expectedAddress
        ) {
            return failure(
                "RESTORE PREFLIGHT BLOCKED — ACTIVE IDENTITY MISMATCH",
            )
        }

        val validationResult =
            try {
                LockedVaultBackupManager.validate(
                    backupJson = backupJson,
                    expectedAddress = expectedAddress,
                )
            } catch (error: Throwable) {
                return failure(
                    "RESTORE PREFLIGHT FAILED — BACKUP VALIDATION ERROR:" +
                        error.javaClass.simpleName,
                )
            }

        if (!validationResult.success) {
            return failure(
                "RESTORE PREFLIGHT FAILED — ${validationResult.status}",
            )
        }

        val backupAddress =
            validationResult.publicAddress
                ?: return failure(
                    "RESTORE PREFLIGHT FAILED — BACKUP IDENTITY MISSING",
                )

        if (
            backupAddress.isBlank() ||
            backupAddress != expectedAddress ||
            backupAddress != activeAddress
        ) {
            return failure(
                "RESTORE PREFLIGHT FAILED — BACKUP IDENTITY MISMATCH",
            )
        }

        val activeVaultAfter =
            vaultStore.loadVault()
                ?: return failure(
                    "RESTORE PREFLIGHT FAILED — ACTIVE VAULT BECAME UNREADABLE",
                )

        if (activeVaultAfter != activeVaultBefore) {
            return failure(
                "RESTORE PREFLIGHT FAILED — ACTIVE VAULT MUTATION DETECTED",
            )
        }

        val activeIdentityAfterResult =
            try {
                NativeBridge.lockedVaultDevnetAddress(
                    activeVaultAfter,
                )
            } catch (error: Throwable) {
                return failure(
                    "RESTORE PREFLIGHT FAILED — POST-CHECK IDENTITY ERROR:" +
                        error.javaClass.simpleName,
                )
            }

        if (!activeIdentityAfterResult.startsWith("ok:")) {
            return failure(
                "RESTORE PREFLIGHT FAILED — POST-CHECK IDENTITY NOT VERIFIED",
            )
        }

        val activeAddressAfter =
            activeIdentityAfterResult.removePrefix("ok:")

        if (
            activeAddressAfter.isBlank() ||
            activeAddressAfter != expectedAddress ||
            activeAddressAfter != activeAddress
        ) {
            return failure(
                "RESTORE PREFLIGHT FAILED — POST-CHECK IDENTITY MISMATCH",
            )
        }

        return PreflightResult(
            success = true,
            publicAddress = backupAddress,
            status =
                "RESTORE PREFLIGHT VERIFIED — NO WRITE PERFORMED",
        )
    }

    private fun failure(
        status: String,
    ): PreflightResult =
        PreflightResult(
            success = false,
            publicAddress = null,
            status = status,
        )
}
