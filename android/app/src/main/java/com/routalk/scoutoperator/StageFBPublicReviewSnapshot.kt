package com.routalk.scoutoperator

internal object StageFBPublicReviewSnapshot {
    internal data class Snapshot(
        val metadata: StageFBCandidateFingerprint.PublicCandidateMetadata,
        val candidateFingerprintSha256: String,
        val reviewReceiptSha256: String,
        val unitsConsumed: Long,
    )

    internal sealed interface Result {
        data class Valid(val snapshot: Snapshot) : Result

        data object Invalid : Result
    }

    fun create(candidate: StageFBPreparedCandidate.Candidate): Result {
        if (candidate.unitsConsumed <= 0L) {
            return Result.Invalid
        }

        val fingerprint = StageFBCandidateFingerprint.derive(candidate.metadata)
        if (
            fingerprint !is StageFBCandidateFingerprint.Result.Valid ||
            fingerprint.fingerprintSha256 != candidate.candidateFingerprintSha256
        ) {
            return Result.Invalid
        }

        val receipt =
            StageFBCandidateReviewReceipt.create(
                metadata = candidate.metadata,
                claimedCandidateFingerprintSha256 = candidate.candidateFingerprintSha256,
            )
        if (
            receipt !is StageFBCandidateReviewReceipt.Result.Valid ||
            receipt.reviewReceiptSha256 != candidate.reviewReceiptSha256
        ) {
            return Result.Invalid
        }

        return Result.Valid(
            Snapshot(
                metadata = candidate.metadata,
                candidateFingerprintSha256 = candidate.candidateFingerprintSha256,
                reviewReceiptSha256 = candidate.reviewReceiptSha256,
                unitsConsumed = candidate.unitsConsumed,
            ),
        )
    }
}
