package com.routalk.scoutoperator

import java.security.MessageDigest

internal object StageFBCandidateFingerprint {
    private const val FIXED_PROGRAM_ID = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"
    private const val FIXED_PAYLOAD = "scout-stage-f-devnet-submission-proof-v1"
    private const val MAX_FEE_LAMPORTS = 10_000L
    private const val MIN_REMAINING_BALANCE_LAMPORTS = 1_000_000L

    internal data class PublicCandidateMetadata(
        val scoutPublicKey: String,
        val expectedSignature: String,
        val recentBlockhash: String,
        val feeLamports: Long,
        val balanceLamports: Long,
        val remainingBalanceLamports: Long,
        val simulationSlot: Long,
        val lastValidBlockHeight: Long,
    )

    internal sealed interface Result {
        data class Valid(
            val fingerprintSha256: String,
            val canonicalReviewText: String,
        ) : Result

        data object Invalid : Result
    }

    fun derive(metadata: PublicCandidateMetadata): Result {
        if (!isValidBase58(metadata.scoutPublicKey, 32, 44)) {
            return Result.Invalid
        }
        if (!isValidBase58(metadata.expectedSignature, 80, 96)) {
            return Result.Invalid
        }
        if (!isValidBase58(metadata.recentBlockhash, 32, 64)) {
            return Result.Invalid
        }
        if (metadata.feeLamports !in 1L..MAX_FEE_LAMPORTS) {
            return Result.Invalid
        }
        if (metadata.balanceLamports <= 0L) {
            return Result.Invalid
        }
        if (metadata.remainingBalanceLamports < MIN_REMAINING_BALANCE_LAMPORTS) {
            return Result.Invalid
        }
        if (metadata.remainingBalanceLamports != metadata.balanceLamports - metadata.feeLamports) {
            return Result.Invalid
        }
        if (metadata.simulationSlot <= 0L || metadata.lastValidBlockHeight <= 0L) {
            return Result.Invalid
        }

        val canonicalReviewText =
            buildString {
                appendLine("scout_stage_fb_candidate_fingerprint_v1")
                appendLine("network=devnet")
                appendLine("program_id=$FIXED_PROGRAM_ID")
                appendLine("payload=$FIXED_PAYLOAD")
                appendLine("scout_public_key=${metadata.scoutPublicKey}")
                appendLine("expected_public_signature=${metadata.expectedSignature}")
                appendLine("recent_blockhash=${metadata.recentBlockhash}")
                appendLine("fee_lamports=${metadata.feeLamports}")
                appendLine("balance_lamports=${metadata.balanceLamports}")
                appendLine("remaining_balance_lamports=${metadata.remainingBalanceLamports}")
                appendLine("simulation_slot=${metadata.simulationSlot}")
                appendLine("last_valid_block_height=${metadata.lastValidBlockHeight}")
            }

        val digest =
            MessageDigest
                .getInstance("SHA-256")
                .digest(canonicalReviewText.toByteArray(Charsets.UTF_8))

        val fingerprint = digest.joinToString(separator = "") { byte -> "%02x".format(byte) }

        return Result.Valid(
            fingerprintSha256 = fingerprint,
            canonicalReviewText = canonicalReviewText,
        )
    }

    private fun isValidBase58(
        value: String,
        minLength: Int,
        maxLength: Int,
    ): Boolean {
        if (value.length !in minLength..maxLength) {
            return false
        }

        return value.all { character -> character in BASE58_ALPHABET }
    }

    private const val BASE58_ALPHABET =
        "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
}
