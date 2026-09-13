package com.routalk.scoutoperator

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class StageFBReadOnlyResolutionTest {
    @Test
    fun missingStatusBeforeExpiryIsPending() {
        val resolution =
            StageFBReadOnlyResolution.resolve(
                observedStatus = null,
                currentBlockHeight = 199L,
                lastValidBlockHeight = 200L,
            )

        assertEquals(StageFBReadOnlyResolution.Resolution.Pending, resolution)
    }

    @Test
    fun missingStatusAtExpiryBoundaryIsStillPending() {
        val resolution =
            StageFBReadOnlyResolution.resolve(
                observedStatus = null,
                currentBlockHeight = 200L,
                lastValidBlockHeight = 200L,
            )

        assertEquals(StageFBReadOnlyResolution.Resolution.Pending, resolution)
    }

    @Test
    fun missingStatusAfterExpiryIsExpired() {
        val resolution =
            StageFBReadOnlyResolution.resolve(
                observedStatus = null,
                currentBlockHeight = 201L,
                lastValidBlockHeight = 200L,
            )

        assertEquals(StageFBReadOnlyResolution.Resolution.Expired, resolution)
    }

    @Test
    fun observedProcessedStatusIsProcessed() {
        val resolution =
            StageFBReadOnlyResolution.resolve(
                observedStatus =
                    StageFBReadOnlyResolution.ObservedStatus(
                        slot = 123L,
                        confirmationState = StageFBReadOnlyResolution.ConfirmationState.PROCESSED,
                        hasExecutionError = false,
                    ),
                currentBlockHeight = 199L,
                lastValidBlockHeight = 200L,
            )

        assertEquals(StageFBReadOnlyResolution.Resolution.Processed(123L), resolution)
    }

    @Test
    fun observedConfirmedStatusIsConfirmed() {
        val resolution =
            StageFBReadOnlyResolution.resolve(
                observedStatus =
                    StageFBReadOnlyResolution.ObservedStatus(
                        slot = 124L,
                        confirmationState = StageFBReadOnlyResolution.ConfirmationState.CONFIRMED,
                        hasExecutionError = false,
                    ),
                currentBlockHeight = 199L,
                lastValidBlockHeight = 200L,
            )

        assertEquals(StageFBReadOnlyResolution.Resolution.Confirmed(124L), resolution)
    }

    @Test
    fun observedFinalizedStatusIsFinalized() {
        val resolution =
            StageFBReadOnlyResolution.resolve(
                observedStatus =
                    StageFBReadOnlyResolution.ObservedStatus(
                        slot = 125L,
                        confirmationState = StageFBReadOnlyResolution.ConfirmationState.FINALIZED,
                        hasExecutionError = false,
                    ),
                currentBlockHeight = 199L,
                lastValidBlockHeight = 200L,
            )

        assertEquals(StageFBReadOnlyResolution.Resolution.Finalized(125L), resolution)
    }

    @Test
    fun executionErrorWinsOverConfirmationState() {
        val resolution =
            StageFBReadOnlyResolution.resolve(
                observedStatus =
                    StageFBReadOnlyResolution.ObservedStatus(
                        slot = 126L,
                        confirmationState = StageFBReadOnlyResolution.ConfirmationState.FINALIZED,
                        hasExecutionError = true,
                    ),
                currentBlockHeight = 199L,
                lastValidBlockHeight = 200L,
            )

        assertEquals(StageFBReadOnlyResolution.Resolution.Failed(126L), resolution)
    }

    @Test
    fun invalidBlockHeightsFailClosed() {
        assertTrue(
            StageFBReadOnlyResolution.resolve(
                observedStatus = null,
                currentBlockHeight = 0L,
                lastValidBlockHeight = 200L,
            ) is StageFBReadOnlyResolution.Resolution.Invalid,
        )
        assertTrue(
            StageFBReadOnlyResolution.resolve(
                observedStatus = null,
                currentBlockHeight = 199L,
                lastValidBlockHeight = 0L,
            ) is StageFBReadOnlyResolution.Resolution.Invalid,
        )
    }

    @Test
    fun invalidObservedSlotFailsClosed() {
        val resolution =
            StageFBReadOnlyResolution.resolve(
                observedStatus =
                    StageFBReadOnlyResolution.ObservedStatus(
                        slot = 0L,
                        confirmationState = StageFBReadOnlyResolution.ConfirmationState.PROCESSED,
                        hasExecutionError = false,
                    ),
                currentBlockHeight = 199L,
                lastValidBlockHeight = 200L,
            )

        assertTrue(resolution is StageFBReadOnlyResolution.Resolution.Invalid)
    }
}
