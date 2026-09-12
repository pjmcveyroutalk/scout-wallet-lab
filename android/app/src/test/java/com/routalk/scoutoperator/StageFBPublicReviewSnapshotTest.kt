package com.routalk.scoutoperator

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class StageFBPublicReviewSnapshotTest {
    @Test
    fun exactPreparedCandidateProducesPublicSnapshot() {
        val candidate = validCandidate()
        val result = StageFBPublicReviewSnapshot.create(candidate)

        assertTrue(result is StageFBPublicReviewSnapshot.Result.Valid)
        result as StageFBPublicReviewSnapshot.Result.Valid
        assertEquals(candidate.metadata, result.snapshot.metadata)
        assertEquals(candidate.candidateFingerprintSha256, result.snapshot.candidateFingerprintSha256)
        assertEquals(candidate.reviewReceiptSha256, result.snapshot.reviewReceiptSha256)
        assertEquals(candidate.unitsConsumed, result.snapshot.unitsConsumed)
    }

    @Test
    fun tamperedFingerprintFailsClosed() {
        val candidate = validCandidate()
        val result =
            StageFBPublicReviewSnapshot.create(
                candidate.copy(
                    candidateFingerprintSha256 = differentSha256(candidate.candidateFingerprintSha256),
                ),
            )

        assertTrue(result is StageFBPublicReviewSnapshot.Result.Invalid)
    }

    @Test
    fun tamperedReviewReceiptFailsClosed() {
        val candidate = validCandidate()
        val result =
            StageFBPublicReviewSnapshot.create(
                candidate.copy(
                    reviewReceiptSha256 = differentSha256(candidate.reviewReceiptSha256),
                ),
            )

        assertTrue(result is StageFBPublicReviewSnapshot.Result.Invalid)
    }

    @Test
    fun invalidSimulationUnitsFailClosed() {
        val candidate = validCandidate()
        val result = StageFBPublicReviewSnapshot.create(candidate.copy(unitsConsumed = 0L))

        assertTrue(result is StageFBPublicReviewSnapshot.Result.Invalid)
    }

    private fun validCandidate(): StageFBPreparedCandidate.Candidate {
        val result =
            StageFBPreparedCandidate.create(
                scoutPublicKey = "1".repeat(32),
                signatureHex = "01".repeat(64),
                recentBlockhash = "1".repeat(32),
                feeLamports = 5_000L,
                balanceLamports = 2_000_000L,
                remainingBalanceLamports = 1_995_000L,
                simulationSlot = 100L,
                unitsConsumed = 321L,
                lastValidBlockHeight = 200L,
                candidateTokenHex = "00".repeat(16),
            )

        require(result is StageFBPreparedCandidate.Result.Valid)
        return result.candidate
    }

    private fun differentSha256(value: String): String {
        val replacement = if (value.first() == '0') '1' else '0'
        return replacement + value.drop(1)
    }
}
