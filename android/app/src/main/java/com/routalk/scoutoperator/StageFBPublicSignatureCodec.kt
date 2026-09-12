package com.routalk.scoutoperator

import java.math.BigInteger

internal object StageFBPublicSignatureCodec {
    fun fromLowercaseHex(signatureHex: String): String? {
        if (
            signatureHex.length != SIGNATURE_HEX_LENGTH ||
            !signatureHex.all { character -> character in LOWERCASE_HEX }
        ) {
            return null
        }

        val bytes = ByteArray(SIGNATURE_BYTES)
        for (index in bytes.indices) {
            val high = hexNibble(signatureHex[index * 2]) ?: return null
            val low = hexNibble(signatureHex[index * 2 + 1]) ?: return null
            bytes[index] = ((high shl 4) or low).toByte()
        }

        var leadingZeroBytes = 0
        while (leadingZeroBytes < bytes.size && bytes[leadingZeroBytes] == 0.toByte()) {
            leadingZeroBytes += 1
        }

        var value = BigInteger(1, bytes)
        val encoded = StringBuilder()
        while (value.signum() > 0) {
            val division = value.divideAndRemainder(BASE)
            encoded.append(BASE58_ALPHABET[division[1].toInt()])
            value = division[0]
        }

        repeat(leadingZeroBytes) {
            encoded.append(BASE58_ALPHABET[0])
        }

        return encoded.reverse().toString()
    }

    private fun hexNibble(character: Char): Int? =
        when (character) {
            in '0'..'9' -> character - '0'
            in 'a'..'f' -> character - 'a' + 10
            else -> null
        }

    private val BASE = BigInteger.valueOf(58L)
    private const val SIGNATURE_BYTES = 64
    private const val SIGNATURE_HEX_LENGTH = SIGNATURE_BYTES * 2
    private const val LOWERCASE_HEX = "0123456789abcdef"
    private const val BASE58_ALPHABET =
        "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
}
