package com.routalk.scoutoperator

internal object PassphrasePolicy {
    const val MIN_LENGTH = 16
    const val MAX_LENGTH = 128

    sealed class ValidationResult {
        data class Valid(
            val passphraseBytes: ByteArray,
        ) : ValidationResult()

        data class Invalid(
            val reason: String,
        ) : ValidationResult()
    }

    fun validateAndEncode(
        passphrase: CharSequence,
        confirmation: CharSequence,
    ): ValidationResult {
        if (passphrase.length < MIN_LENGTH) {
            return ValidationResult.Invalid(
                "Passphrase must be at least $MIN_LENGTH characters.",
            )
        }

        if (passphrase.length > MAX_LENGTH) {
            return ValidationResult.Invalid(
                "Passphrase must be no more than $MAX_LENGTH characters.",
            )
        }

        if (!matches(passphrase, confirmation)) {
            return ValidationResult.Invalid(
                "Passphrases do not match.",
            )
        }

        val passphraseBytes = ByteArray(passphrase.length)

        for (index in passphrase.indices) {
            val character = passphrase[index]

            if (character.code !in 0x20..0x7E) {
                passphraseBytes.fill(0)

                return ValidationResult.Invalid(
                    "Passphrase must use printable ASCII characters only.",
                )
            }

            passphraseBytes[index] = character.code.toByte()
        }

        return ValidationResult.Valid(passphraseBytes)
    }

    fun wipe(bytes: ByteArray) {
        bytes.fill(0)
    }

    private fun matches(
        first: CharSequence,
        second: CharSequence,
    ): Boolean {
        if (first.length != second.length) {
            return false
        }

        var difference = 0

        for (index in first.indices) {
            difference =
                difference or
                    (first[index].code xor second[index].code)
        }

        return difference == 0
    }
}
