package com.routalk.scoutoperator

internal object StageFBPublicBindingCheck {
    internal sealed interface Result {
        data class Bound(
            val candidateFingerprintSha256: String,
            val reviewReceiptSha256: String,
        ) : Result

        data object Invalid : Result
    }

    fun verify(
        metadata: StageFBCandidateFingerprint.PublicCandidateMetadata,
        claimedCandidateFingerprintSha256: String,
        claimedReviewReceiptSha256: String,
        guardRecord: StageFBAttemptGuard.Record,
    ): Result {
        if (!isLowercaseSha256(claimedReviewReceiptSha256)) {
            return Result.Invalid
        }

        val receipt =
            StageFBCandidateReviewReceipt.create(
                metadata = metadata,
                claimedCandidateFingerprintSha256 = claimedCandidateFingerprintSha256,
            )
        if (receipt !is StageFBCandidateReviewReceipt.Result.Valid) {
            return Result.Invalid
        }
        if (receipt.reviewReceiptSha256 != claimedReviewReceiptSha256) {
            return Result.Invalid
        }

        if (!guardRecord.attemptStarted) {
            return Result.Invalid
        }
        if (guardRecord.expectedSignature != metadata.expectedSignature) {
            return Result.Invalid
        }
        if (guardRecord.lastValidBlockHeight != metadata.lastValidBlockHeight) {
            return Result.Invalid
        }

        return Result.Bound(
            candidateFingerprintSha256 = receipt.candidateFingerprintSha256,
            reviewReceiptSha256 = receipt.reviewReceiptSha256,
        )
    }

    private fun isLowercaseSha256(value: String): Boolean =
        value.length == SHA256_HEX_LENGTH && value.all { character -> character in LOWERCASE_HEX }

    private const val SHA256_HEX_LENGTH = 64
    private const val LOWERCASE_HEX = "0123456789abcdef"
}
