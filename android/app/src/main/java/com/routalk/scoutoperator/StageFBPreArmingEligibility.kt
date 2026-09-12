package com.routalk.scoutoperator

internal object StageFBPreArmingEligibility {
    internal sealed interface Result {
        data class Eligible(
            val candidateFingerprintSha256: String,
            val reviewReceiptSha256: String,
            val expectedSignature: String,
            val lastValidBlockHeight: Long,
        ) : Result

        data object Ineligible : Result
    }

    fun evaluate(
        metadata: StageFBCandidateFingerprint.PublicCandidateMetadata,
        claimedCandidateFingerprintSha256: String,
        claimedReviewReceiptSha256: String,
        guardRecord: StageFBAttemptGuard.Record,
        currentBlockHeight: Long,
    ): Result {
        if (currentBlockHeight <= 0L) {
            return Result.Ineligible
        }
        if (guardRecord.publicStatus != StageFBAttemptGuard.PublicStatus.ATTEMPT_STARTED) {
            return Result.Ineligible
        }
        if (currentBlockHeight > guardRecord.lastValidBlockHeight) {
            return Result.Ineligible
        }

        val binding =
            StageFBPublicBindingCheck.verify(
                metadata = metadata,
                claimedCandidateFingerprintSha256 = claimedCandidateFingerprintSha256,
                claimedReviewReceiptSha256 = claimedReviewReceiptSha256,
                guardRecord = guardRecord,
            )
        if (binding !is StageFBPublicBindingCheck.Result.Bound) {
            return Result.Ineligible
        }

        return Result.Eligible(
            candidateFingerprintSha256 = binding.candidateFingerprintSha256,
            reviewReceiptSha256 = binding.reviewReceiptSha256,
            expectedSignature = metadata.expectedSignature,
            lastValidBlockHeight = guardRecord.lastValidBlockHeight,
        )
    }
}
