package com.routalk.scoutoperator

internal object StageFBReadOnlyResolution {
    internal enum class ConfirmationState {
        PROCESSED,
        CONFIRMED,
        FINALIZED,
    }

    internal data class ObservedStatus(
        val slot: Long,
        val confirmationState: ConfirmationState,
        val hasExecutionError: Boolean,
    )

    internal sealed interface Resolution {
        data object Pending : Resolution

        data class Processed(val slot: Long) : Resolution

        data class Confirmed(val slot: Long) : Resolution

        data class Finalized(val slot: Long) : Resolution

        data class Failed(val slot: Long) : Resolution

        data object Expired : Resolution

        data object Invalid : Resolution
    }

    fun resolve(
        observedStatus: ObservedStatus?,
        currentBlockHeight: Long,
        lastValidBlockHeight: Long,
    ): Resolution {
        if (currentBlockHeight <= 0L || lastValidBlockHeight <= 0L) {
            return Resolution.Invalid
        }

        if (observedStatus == null) {
            return if (currentBlockHeight > lastValidBlockHeight) {
                Resolution.Expired
            } else {
                Resolution.Pending
            }
        }

        if (observedStatus.slot <= 0L) {
            return Resolution.Invalid
        }

        if (observedStatus.hasExecutionError) {
            return Resolution.Failed(observedStatus.slot)
        }

        return when (observedStatus.confirmationState) {
            ConfirmationState.PROCESSED -> Resolution.Processed(observedStatus.slot)
            ConfirmationState.CONFIRMED -> Resolution.Confirmed(observedStatus.slot)
            ConfirmationState.FINALIZED -> Resolution.Finalized(observedStatus.slot)
        }
    }
}
