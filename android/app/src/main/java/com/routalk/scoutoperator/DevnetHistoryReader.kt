package com.routalk.scoutoperator

import android.content.Context

internal object DevnetHistoryReader {
    private const val HISTORY_LIMIT = 10

    internal data class SignatureRecord(
        val signature: String,
        val slot: String,
    )

    internal data class HistoryResult(
        val success: Boolean,
        val records: List<SignatureRecord>,
        val status: String,
    )

    fun refresh(
        context: Context,
        expectedAddress: String,
    ): HistoryResult {
        if (expectedAddress.isBlank()) {
            return failure(
                "HISTORY REFRESH BLOCKED — VERIFIED IDENTITY MISSING",
            )
        }

        val vaultStore =
            LockedVaultStore(context)

        if (!vaultStore.hasVault()) {
            return failure(
                "HISTORY REFRESH BLOCKED — ENCRYPTED VAULT MISSING",
            )
        }

        val lockedVaultJson =
            vaultStore.loadVault()
                ?: return failure(
                    "HISTORY REFRESH BLOCKED — ENCRYPTED VAULT UNREADABLE",
                )

        val identityResult =
            NativeBridge.lockedVaultDevnetAddress(
                lockedVaultJson,
            )

        if (!identityResult.startsWith("ok:")) {
            return failure(
                "HISTORY REFRESH BLOCKED — IDENTITY NOT VERIFIED",
            )
        }

        val returnedAddress =
            identityResult.removePrefix("ok:")

        if (
            returnedAddress.isBlank() ||
            returnedAddress != expectedAddress
        ) {
            return failure(
                "HISTORY REFRESH BLOCKED — IDENTITY MISMATCH",
            )
        }

        val nativeResult =
            NativeBridge.lockedVaultDevnetHistory(
                lockedVaultJson,
            )

        if (!nativeResult.startsWith("ok:")) {
            return failure(
                "HISTORY REFRESH FAILED — $nativeResult",
            )
        }

        val encodedHistory =
            nativeResult.removePrefix("ok:")

        if (encodedHistory.isBlank()) {
            return HistoryResult(
                success = true,
                records = emptyList(),
                status = "VERIFIED — NO DEVNET HISTORY",
            )
        }

        val encodedRecords =
            encodedHistory.split(",")

        if (encodedRecords.size > HISTORY_LIMIT) {
            return failure(
                "HISTORY REFRESH FAILED — HISTORY LIMIT EXCEEDED",
            )
        }

        val records =
            mutableListOf<SignatureRecord>()

        for (encodedRecord in encodedRecords) {
            val parts =
                encodedRecord.split(
                    ":",
                    limit = 2,
                )

            if (parts.size != 2) {
                return failure(
                    "HISTORY REFRESH FAILED — INVALID NATIVE RESPONSE",
                )
            }

            val signature =
                parts[0]

            val slot =
                parts[1]

            if (
                signature.isBlank() ||
                slot.isBlank() ||
                !slot.all { character ->
                    character in '0'..'9'
                }
            ) {
                return failure(
                    "HISTORY REFRESH FAILED — INVALID HISTORY RECORD",
                )
            }

            records.add(
                SignatureRecord(
                    signature = signature,
                    slot = slot,
                ),
            )
        }

        return HistoryResult(
            success = true,
            records = records.toList(),
            status = "VERIFIED — DEVNET HISTORY",
        )
    }

    private fun failure(
        status: String,
    ): HistoryResult =
        HistoryResult(
            success = false,
            records = emptyList(),
            status = status,
        )
}
