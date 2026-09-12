package com.routalk.scoutoperator

import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

internal object StageFBReadOnlyStatusClient {
    internal sealed interface Result {
        data object Missing : Result

        data class Observed(
            val slot: Long,
            val confirmationState: StageFBReadOnlyResolution.ConfirmationState,
            val hasExecutionError: Boolean,
        ) : Result

        data object Invalid : Result
    }

    fun fetch(expectedSignature: String): Result {
        if (!isValidPublicSignature(expectedSignature)) {
            return Result.Invalid
        }

        val endpoint = NativeBridge.rpcEndpoint()
        if (endpoint != DEVNET_RPC_ENDPOINT) {
            return Result.Invalid
        }

        val requestBody =
            JSONObject()
                .put("jsonrpc", "2.0")
                .put("id", REQUEST_ID)
                .put("method", "getSignatureStatuses")
                .put(
                    "params",
                    JSONArray()
                        .put(JSONArray().put(expectedSignature))
                        .put(JSONObject().put("searchTransactionHistory", true)),
                )
                .toString()

        val connection =
            (URL(endpoint).openConnection() as? HttpURLConnection)
                ?: return Result.Invalid

        return try {
            connection.requestMethod = "POST"
            connection.connectTimeout = TIMEOUT_MILLIS
            connection.readTimeout = TIMEOUT_MILLIS
            connection.doOutput = true
            connection.setRequestProperty("Content-Type", "application/json")

            connection.outputStream.use { output ->
                output.write(requestBody.toByteArray(Charsets.UTF_8))
            }

            if (connection.responseCode !in 200..299) {
                return Result.Invalid
            }

            val responseBody =
                connection.inputStream.bufferedReader(Charsets.UTF_8).use { reader -> reader.readText() }
            parse(responseBody)
        } catch (_: Throwable) {
            Result.Invalid
        } finally {
            connection.disconnect()
        }
    }

    internal fun parse(responseBody: String): Result {
        val response =
            try {
                JSONObject(responseBody)
            } catch (_: Throwable) {
                return Result.Invalid
            }

        if (response.has("error") && !response.isNull("error")) {
            return Result.Invalid
        }

        val result = response.optJSONObject("result") ?: return Result.Invalid
        val values = result.optJSONArray("value") ?: return Result.Invalid
        if (values.length() != 1) {
            return Result.Invalid
        }
        if (values.isNull(0)) {
            return Result.Missing
        }

        val status = values.optJSONObject(0) ?: return Result.Invalid
        val slot = status.optLong("slot", 0L)
        if (slot <= 0L) {
            return Result.Invalid
        }

        val confirmationState =
            when (status.optString("confirmationStatus", "")) {
                "processed" -> StageFBReadOnlyResolution.ConfirmationState.PROCESSED
                "confirmed" -> StageFBReadOnlyResolution.ConfirmationState.CONFIRMED
                "finalized" -> StageFBReadOnlyResolution.ConfirmationState.FINALIZED
                else -> return Result.Invalid
            }

        return Result.Observed(
            slot = slot,
            confirmationState = confirmationState,
            hasExecutionError = status.has("err") && !status.isNull("err"),
        )
    }

    private fun isValidPublicSignature(value: String): Boolean {
        if (value.length !in MIN_SIGNATURE_LENGTH..MAX_SIGNATURE_LENGTH) {
            return false
        }
        return value.all { character -> character in BASE58_ALPHABET }
    }

    private const val DEVNET_RPC_ENDPOINT = "https://api.devnet.solana.com"
    private const val REQUEST_ID = 41
    private const val TIMEOUT_MILLIS = 10_000
    private const val MIN_SIGNATURE_LENGTH = 80
    private const val MAX_SIGNATURE_LENGTH = 96
    private const val BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
}
