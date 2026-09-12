package com.routalk.scoutoperator

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class StageFBPreparedCandidateTest {
    @Test
    fun signatureCodecMatchesKnownBase58Vector() {
        val encoded = StageFBPublicSignatureCodec.fromLowercaseHex("01".repeat(64))

        assertEquals(
            "2AXDGYSE4f2sz7tvMMzyHvUfcoJmxudvdhBcmiUSo6ijwfYmfZYsKRxboQMPh3R4kUhXRVdtSXFXMheka4Rc4P2",
            encoded,
        )
    }

    @Test
    fun signatureCodecRejectsMalformedNativeHex() {
        assertEquals(null, StageFBPublicSignatureCodec.fromLowercaseHex("01".repeat(63)))
        assertEquals(null, StageFBPublicSignatureCodec.fromLowercaseHex("AB".repeat(64)))
    }

    @Test
    fun nativePresubmitFieldsProduceReviewedCandidateBinding() {
        val result = validCandidate()

        assertTrue(result is StageFBPreparedCandidate.Result.Valid)
        result as StageFBPreparedCandidate.Result.Valid
        assertEquals(87, result.candidate.metadata.expectedSignature.length)
        assertEquals(64, result.candidate.candidateFingerprintSha256.length)
        assertEquals(64, result.candidate.reviewReceiptSha256.length)
        assertEquals("00".repeat(16), result.candidate.tokenHex)
        assertEquals(321L, result.candidate.unitsConsumed)
    }

    @Test
    fun malformedCandidateTokenFailsClosed() {
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
                candidateTokenHex = "00".repeat(15),
            )

        assertTrue(result is StageFBPreparedCandidate.Result.Invalid)
    }

    private fun validCandidate(): StageFBPreparedCandidate.Result =
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
}
