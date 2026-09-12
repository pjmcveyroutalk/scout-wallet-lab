package com.routalk.scoutoperator

import java.security.MessageDigest

internal object StageFBCandidateReviewReceipt {
    internal sealed interface Result {
        data class Valid(
            val candidateFingerprintSha256: String,
            val reviewReceiptSha256: String,
            val canonicalReviewReceiptText: String,
        ) : Result

        data object Invalid : Result
    }

    fun create(
        metadata: StageFBCandidateFingerprint.PublicCandidateMetadata,
        claimedCandidateFingerprintSha256: String,
    ): Result {
        if (!isLowercaseSha256(claimedCandidateFingerprintSha256)) {
            return Result.Invalid
        }

        val derived = StageFBCandidateFingerprint.derive(metadata)
        if (derived !is StageFBCandidateFingerprint.Result.Valid) {
            return Result.Invalid
        }
        if (derived.fingerprintSha256 != claimedCandidateFingerprintSha256) {
            return Result.Invalid
        }

        val canonicalReviewReceiptText =
            buildString {
                appendLine("scout_stage_fb_candidate_review_receipt_v1")
                appendLine("candidate_fingerprint_sha256=$claimedCandidateFingerprintSha256")
                appendLine("review_scope=public_metadata_only")
                appendLine("execution_authorized=false")
                appendLine("ledger_write_armed=false")
            }

        val digest =
            MessageDigest
                .getInstance("SHA-256")
                .digest(canonicalReviewReceiptText.toByteArray(Charsets.UTF_8))

        val reviewReceiptSha256 =
            digest.joinToString(separator = "") { byte -> "%02x".format(byte) }

        return Result.Valid(
            candidateFingerprintSha256 = claimedCandidateFingerprintSha256,
            reviewReceiptSha256 = reviewReceiptSha256,
            canonicalReviewReceiptText = canonicalReviewReceiptText,
        )
    }

    private fun isLowercaseSha256(value: String): Boolean =
        value.length == SHA256_HEX_LENGTH && value.all { character -> character in LOWERCASE_HEX }

    private const val SHA256_HEX_LENGTH = 64
    private const val LOWERCASE_HEX = "0123456789abcdef"
}
