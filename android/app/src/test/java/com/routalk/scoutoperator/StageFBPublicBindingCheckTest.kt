package com.routalk.scoutoperator

import org.junit.Assert.assertTrue
import org.junit.Test

class StageFBPublicBindingCheckTest {
    @Test
    fun exactReviewedCandidateIsBound() {
        val fixture = fixture()

        val result =
            StageFBPublicBindingCheck.verify(
                metadata = fixture.metadata,
                claimedCandidateFingerprintSha256 = fixture.candidateFingerprintSha256,
                claimedReviewReceiptSha256 = fixture.reviewReceiptSha256,
                guardRecord = fixture.guardRecord,
            )

        assertTrue(result is StageFBPublicBindingCheck.Result.Bound)
    }

    @Test
    fun mismatchedPersistedCandidateFingerprintFailsClosed() {
        val fixture = fixture()

        val result =
            StageFBPublicBindingCheck.verify(
                metadata = fixture.metadata,
                claimedCandidateFingerprintSha256 = fixture.candidateFingerprintSha256,
                claimedReviewReceiptSha256 = fixture.reviewReceiptSha256,
                guardRecord =
                    fixture.guardRecord.copy(
                        candidateFingerprintSha256 = differentSha256(fixture.candidateFingerprintSha256),
                    ),
            )

        assertTrue(result is StageFBPublicBindingCheck.Result.Invalid)
    }

    @Test
    fun mismatchedPersistedReviewReceiptFailsClosed() {
        val fixture = fixture()

        val result =
            StageFBPublicBindingCheck.verify(
                metadata = fixture.metadata,
                claimedCandidateFingerprintSha256 = fixture.candidateFingerprintSha256,
                claimedReviewReceiptSha256 = fixture.reviewReceiptSha256,
                guardRecord =
                    fixture.guardRecord.copy(
                        reviewReceiptSha256 = differentSha256(fixture.reviewReceiptSha256),
                    ),
            )

        assertTrue(result is StageFBPublicBindingCheck.Result.Invalid)
    }

    @Test
    fun mismatchedPersistedSignatureFailsClosed() {
        val fixture = fixture()

        val result =
            StageFBPublicBindingCheck.verify(
                metadata = fixture.metadata,
                claimedCandidateFingerprintSha256 = fixture.candidateFingerprintSha256,
                claimedReviewReceiptSha256 = fixture.reviewReceiptSha256,
                guardRecord = fixture.guardRecord.copy(expectedSignature = "2".repeat(88)),
            )

        assertTrue(result is StageFBPublicBindingCheck.Result.Invalid)
    }

    @Test
    fun mismatchedPersistedLastValidBlockHeightFailsClosed() {
        val fixture = fixture()

        val result =
            StageFBPublicBindingCheck.verify(
                metadata = fixture.metadata,
                claimedCandidateFingerprintSha256 = fixture.candidateFingerprintSha256,
                claimedReviewReceiptSha256 = fixture.reviewReceiptSha256,
                guardRecord =
                    fixture.guardRecord.copy(
                        lastValidBlockHeight = fixture.metadata.lastValidBlockHeight + 1L,
                    ),
            )

        assertTrue(result is StageFBPublicBindingCheck.Result.Invalid)
    }

    @Test
    fun missingAttemptStartedFailsClosed() {
        val fixture = fixture()

        val result =
            StageFBPublicBindingCheck.verify(
                metadata = fixture.metadata,
                claimedCandidateFingerprintSha256 = fixture.candidateFingerprintSha256,
                claimedReviewReceiptSha256 = fixture.reviewReceiptSha256,
                guardRecord = fixture.guardRecord.copy(attemptStarted = false),
            )

        assertTrue(result is StageFBPublicBindingCheck.Result.Invalid)
    }

    @Test
    fun malformedClaimedReviewReceiptFailsClosed() {
        val fixture = fixture()

        val result =
            StageFBPublicBindingCheck.verify(
                metadata = fixture.metadata,
                claimedCandidateFingerprintSha256 = fixture.candidateFingerprintSha256,
                claimedReviewReceiptSha256 = "not-a-sha256",
                guardRecord = fixture.guardRecord,
            )

        assertTrue(result is StageFBPublicBindingCheck.Result.Invalid)
    }

    @Test
    fun mismatchedClaimedCandidateFingerprintFailsClosed() {
        val fixture = fixture()

        val result =
            StageFBPublicBindingCheck.verify(
                metadata = fixture.metadata,
                claimedCandidateFingerprintSha256 = differentSha256(fixture.candidateFingerprintSha256),
                claimedReviewReceiptSha256 = fixture.reviewReceiptSha256,
                guardRecord = fixture.guardRecord,
            )

        assertTrue(result is StageFBPublicBindingCheck.Result.Invalid)
    }

    private fun fixture(): Fixture {
        val metadata =
            StageFBCandidateFingerprint.PublicCandidateMetadata(
                scoutPublicKey = "1".repeat(32),
                expectedSignature = "1".repeat(88),
                recentBlockhash = "1".repeat(32),
                feeLamports = 5_000L,
                balanceLamports = 2_000_000L,
                remainingBalanceLamports = 1_995_000L,
                simulationSlot = 100L,
                lastValidBlockHeight = 200L,
            )

        val fingerprint = StageFBCandidateFingerprint.derive(metadata)
        check(fingerprint is StageFBCandidateFingerprint.Result.Valid)

        val receipt =
            StageFBCandidateReviewReceipt.create(
                metadata = metadata,
                claimedCandidateFingerprintSha256 = fingerprint.fingerprintSha256,
            )
        check(receipt is StageFBCandidateReviewReceipt.Result.Valid)

        return Fixture(
            metadata = metadata,
            candidateFingerprintSha256 = fingerprint.fingerprintSha256,
            reviewReceiptSha256 = receipt.reviewReceiptSha256,
            guardRecord =
                StageFBAttemptGuard.Record(
                    candidateFingerprintSha256 = fingerprint.fingerprintSha256,
                    reviewReceiptSha256 = receipt.reviewReceiptSha256,
                    expectedSignature = metadata.expectedSignature,
                    attemptStarted = true,
                    lastValidBlockHeight = metadata.lastValidBlockHeight,
                    publicStatus = StageFBAttemptGuard.PublicStatus.ATTEMPT_STARTED,
                ),
        )
    }

    private fun differentSha256(value: String): String =
        if (value.first() == '0') {
            "1${value.drop(1)}"
        } else {
            "0${value.drop(1)}"
        }

    private data class Fixture(
        val metadata: StageFBCandidateFingerprint.PublicCandidateMetadata,
        val candidateFingerprintSha256: String,
        val reviewReceiptSha256: String,
        val guardRecord: StageFBAttemptGuard.Record,
    )
}
