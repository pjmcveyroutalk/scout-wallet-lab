package com.routalk.scoutoperator

internal object StageFBPreparedCandidate {
    internal data class Candidate(
        val tokenHex: String,
        val metadata: StageFBCandidateFingerprint.PublicCandidateMetadata,
        val candidateFingerprintSha256: String,
        val reviewReceiptSha256: String,
        val unitsConsumed: Long,
    )

    internal sealed interface Result {
        data class Valid(val candidate: Candidate) : Result

        data object Invalid : Result
    }

    fun create(
        scoutPublicKey: String,
        signatureHex: String,
        recentBlockhash: String,
        feeLamports: Long,
        balanceLamports: Long,
        remainingBalanceLamports: Long,
        simulationSlot: Long,
        unitsConsumed: Long,
        lastValidBlockHeight: Long,
        candidateTokenHex: String,
    ): Result {
        if (
            candidateTokenHex.length != CANDIDATE_TOKEN_HEX_LENGTH ||
            !candidateTokenHex.all { character -> character in LOWERCASE_HEX } ||
            unitsConsumed <= 0L
        ) {
            return Result.Invalid
        }

        val expectedSignature =
            StageFBPublicSignatureCodec.fromLowercaseHex(signatureHex)
                ?: return Result.Invalid

        val metadata =
            StageFBCandidateFingerprint.PublicCandidateMetadata(
                scoutPublicKey = scoutPublicKey,
                expectedSignature = expectedSignature,
                recentBlockhash = recentBlockhash,
                feeLamports = feeLamports,
                balanceLamports = balanceLamports,
                remainingBalanceLamports = remainingBalanceLamports,
                simulationSlot = simulationSlot,
                lastValidBlockHeight = lastValidBlockHeight,
            )

        val fingerprint = StageFBCandidateFingerprint.derive(metadata)
        if (fingerprint !is StageFBCandidateFingerprint.Result.Valid) {
            return Result.Invalid
        }

        val receipt =
            StageFBCandidateReviewReceipt.create(
                metadata = metadata,
                claimedCandidateFingerprintSha256 = fingerprint.fingerprintSha256,
            )
        if (receipt !is StageFBCandidateReviewReceipt.Result.Valid) {
            return Result.Invalid
        }

        return Result.Valid(
            Candidate(
                tokenHex = candidateTokenHex,
                metadata = metadata,
                candidateFingerprintSha256 = fingerprint.fingerprintSha256,
                reviewReceiptSha256 = receipt.reviewReceiptSha256,
                unitsConsumed = unitsConsumed,
            ),
        )
    }

    private const val CANDIDATE_TOKEN_HEX_LENGTH = 32
    private const val LOWERCASE_HEX = "0123456789abcdef"
}
