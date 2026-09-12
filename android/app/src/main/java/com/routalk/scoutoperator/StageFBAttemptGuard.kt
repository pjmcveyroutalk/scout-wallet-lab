package com.routalk.scoutoperator

import android.content.Context
import android.content.SharedPreferences

internal class StageFBAttemptGuard(context: Context) {
    internal enum class PublicStatus {
        ATTEMPT_STARTED,
        PROCESSED,
        CONFIRMED,
        FINALIZED,
        FAILED,
        EXPIRED,
    }

    internal data class Record(
        val expectedSignature: String,
        val attemptStarted: Boolean,
        val lastValidBlockHeight: Long,
        val publicStatus: PublicStatus,
    )

    internal sealed interface LoadResult {
        data object Empty : LoadResult

        data class Present(val record: Record) : LoadResult

        data object Corrupt : LoadResult
    }

    internal enum class BeginResult {
        STARTED_AND_PERSISTED,
        ALREADY_GUARDED,
        INVALID_PUBLIC_METADATA,
        PERSISTENCE_FAILED,
    }

    internal enum class UpdateResult {
        UPDATED_AND_PERSISTED,
        NO_GUARD_PRESENT,
        CORRUPT_GUARD,
        INVALID_TRANSITION,
        PERSISTENCE_FAILED,
    }

    private val preferences: SharedPreferences =
        context.applicationContext.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)

    fun load(): LoadResult {
        if (!preferences.contains(KEY_ATTEMPT_STARTED)) {
            return LoadResult.Empty
        }

        val expectedSignature = preferences.getString(KEY_EXPECTED_SIGNATURE, null)
        val attemptStarted = preferences.getBoolean(KEY_ATTEMPT_STARTED, false)
        val lastValidBlockHeight = preferences.getLong(KEY_LAST_VALID_BLOCK_HEIGHT, 0L)
        val rawStatus = preferences.getString(KEY_PUBLIC_STATUS, null)
        val publicStatus = rawStatus?.let(::parseStatus)

        if (
            expectedSignature == null ||
            !isValidPublicSignature(expectedSignature) ||
            !attemptStarted ||
            lastValidBlockHeight <= 0L ||
            publicStatus == null
        ) {
            return LoadResult.Corrupt
        }

        return LoadResult.Present(
            Record(
                expectedSignature = expectedSignature,
                attemptStarted = true,
                lastValidBlockHeight = lastValidBlockHeight,
                publicStatus = publicStatus,
            ),
        )
    }

    fun beginAttempt(
        expectedSignature: String,
        lastValidBlockHeight: Long,
    ): BeginResult {
        if (!isValidPublicSignature(expectedSignature) || lastValidBlockHeight <= 0L) {
            return BeginResult.INVALID_PUBLIC_METADATA
        }

        if (load() != LoadResult.Empty) {
            return BeginResult.ALREADY_GUARDED
        }

        val committed =
            preferences
                .edit()
                .putString(KEY_EXPECTED_SIGNATURE, expectedSignature)
                .putBoolean(KEY_ATTEMPT_STARTED, true)
                .putLong(KEY_LAST_VALID_BLOCK_HEIGHT, lastValidBlockHeight)
                .putString(KEY_PUBLIC_STATUS, PublicStatus.ATTEMPT_STARTED.name)
                .commit()

        if (!committed) {
            return BeginResult.PERSISTENCE_FAILED
        }

        val persisted = load()
        return if (
            persisted is LoadResult.Present &&
            persisted.record.expectedSignature == expectedSignature &&
            persisted.record.attemptStarted &&
            persisted.record.lastValidBlockHeight == lastValidBlockHeight &&
            persisted.record.publicStatus == PublicStatus.ATTEMPT_STARTED
        ) {
            BeginResult.STARTED_AND_PERSISTED
        } else {
            BeginResult.PERSISTENCE_FAILED
        }
    }

    fun updateFromResolution(
        expectedSignature: String,
        resolution: StageFBReadOnlyResolution.Resolution,
    ): UpdateResult {
        val loaded = load()
        if (loaded == LoadResult.Empty) {
            return UpdateResult.NO_GUARD_PRESENT
        }
        if (loaded == LoadResult.Corrupt) {
            return UpdateResult.CORRUPT_GUARD
        }

        loaded as LoadResult.Present
        if (loaded.record.expectedSignature != expectedSignature) {
            return UpdateResult.INVALID_TRANSITION
        }

        val nextStatus =
            when (resolution) {
                StageFBReadOnlyResolution.Resolution.Pending -> PublicStatus.ATTEMPT_STARTED
                is StageFBReadOnlyResolution.Resolution.Processed -> PublicStatus.PROCESSED
                is StageFBReadOnlyResolution.Resolution.Confirmed -> PublicStatus.CONFIRMED
                is StageFBReadOnlyResolution.Resolution.Finalized -> PublicStatus.FINALIZED
                is StageFBReadOnlyResolution.Resolution.Failed -> PublicStatus.FAILED
                StageFBReadOnlyResolution.Resolution.Expired -> PublicStatus.EXPIRED
                StageFBReadOnlyResolution.Resolution.Invalid -> return UpdateResult.INVALID_TRANSITION
            }

        if (!isAllowedTransition(loaded.record.publicStatus, nextStatus)) {
            return UpdateResult.INVALID_TRANSITION
        }

        if (loaded.record.publicStatus == nextStatus) {
            return UpdateResult.UPDATED_AND_PERSISTED
        }

        val committed =
            preferences
                .edit()
                .putString(KEY_PUBLIC_STATUS, nextStatus.name)
                .commit()

        if (!committed) {
            return UpdateResult.PERSISTENCE_FAILED
        }

        val persisted = load()
        return if (
            persisted is LoadResult.Present &&
            persisted.record.expectedSignature == expectedSignature &&
            persisted.record.publicStatus == nextStatus
        ) {
            UpdateResult.UPDATED_AND_PERSISTED
        } else {
            UpdateResult.PERSISTENCE_FAILED
        }
    }

    private fun isAllowedTransition(
        current: PublicStatus,
        next: PublicStatus,
    ): Boolean {
        if (current == next) {
            return true
        }

        return when (current) {
            PublicStatus.ATTEMPT_STARTED ->
                next == PublicStatus.PROCESSED ||
                    next == PublicStatus.CONFIRMED ||
                    next == PublicStatus.FINALIZED ||
                    next == PublicStatus.FAILED ||
                    next == PublicStatus.EXPIRED

            PublicStatus.PROCESSED ->
                next == PublicStatus.CONFIRMED ||
                    next == PublicStatus.FINALIZED ||
                    next == PublicStatus.FAILED

            PublicStatus.CONFIRMED ->
                next == PublicStatus.FINALIZED || next == PublicStatus.FAILED

            PublicStatus.FINALIZED,
            PublicStatus.FAILED,
            PublicStatus.EXPIRED,
            -> false
        }
    }

    private fun parseStatus(value: String): PublicStatus? =
        PublicStatus.entries.firstOrNull { status -> status.name == value }

    private fun isValidPublicSignature(value: String): Boolean {
        if (value.length !in MIN_SIGNATURE_LENGTH..MAX_SIGNATURE_LENGTH) {
            return false
        }

        return value.all { character -> character in BASE58_ALPHABET }
    }

    private companion object {
        const val PREFERENCES_NAME = "scout_stage_fb_attempt_guard_v1"
        const val KEY_EXPECTED_SIGNATURE = "expected_public_signature"
        const val KEY_ATTEMPT_STARTED = "attempt_started"
        const val KEY_LAST_VALID_BLOCK_HEIGHT = "last_valid_block_height"
        const val KEY_PUBLIC_STATUS = "final_public_status"

        const val MIN_SIGNATURE_LENGTH = 80
        const val MAX_SIGNATURE_LENGTH = 96
        const val BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
    }
}
