package com.routalk.scoutoperator

import android.content.Context

internal class LockedVaultStore(context: Context) {
    private val preferences =
        context.applicationContext.getSharedPreferences(
            PREFERENCES_NAME,
            Context.MODE_PRIVATE,
        )

    fun hasVault(): Boolean =
        preferences.contains(KEY_LOCKED_VAULT)

    fun loadVault(): String? =
        preferences.getString(KEY_LOCKED_VAULT, null)
            ?.takeIf { it.isNotBlank() }

    fun saveVault(vaultJson: String): Boolean {
        if (!looksLikeLockedVaultJson(vaultJson)) {
            return false
        }

        return preferences
            .edit()
            .putString(KEY_LOCKED_VAULT, vaultJson)
            .commit()
    }

    fun prepareRekeyReplacement(
        expectedCurrentVaultJson: String,
        replacementVaultJson: String,
    ): Boolean {
        if (
            !looksLikeLockedVaultJson(expectedCurrentVaultJson) ||
            !looksLikeLockedVaultJson(replacementVaultJson)
        ) {
            return false
        }

        val currentVaultJson = loadVault() ?: return false

        if (currentVaultJson != expectedCurrentVaultJson) {
            return false
        }

        return preferences
            .edit()
            .putString(KEY_REKEY_ROLLBACK_VAULT, currentVaultJson)
            .putString(KEY_LOCKED_VAULT, replacementVaultJson)
            .commit()
    }

    fun rollbackRekey(expectedReplacementVaultJson: String): Boolean {
        if (!looksLikeLockedVaultJson(expectedReplacementVaultJson)) {
            return false
        }

        val currentVaultJson = loadVault() ?: return false
        val rollbackVaultJson =
            preferences
                .getString(KEY_REKEY_ROLLBACK_VAULT, null)
                ?.takeIf { it.isNotBlank() }
                ?: return false

        if (
            currentVaultJson != expectedReplacementVaultJson ||
            !looksLikeLockedVaultJson(rollbackVaultJson)
        ) {
            return false
        }

        return preferences
            .edit()
            .putString(KEY_LOCKED_VAULT, rollbackVaultJson)
            .remove(KEY_REKEY_ROLLBACK_VAULT)
            .commit()
    }

    fun finalizeRekey(expectedReplacementVaultJson: String): Boolean {
        if (!looksLikeLockedVaultJson(expectedReplacementVaultJson)) {
            return false
        }

        val currentVaultJson = loadVault() ?: return false

        if (currentVaultJson != expectedReplacementVaultJson) {
            return false
        }

        return preferences
            .edit()
            .remove(KEY_REKEY_ROLLBACK_VAULT)
            .commit()
    }

    fun clearVault(): Boolean =
        preferences
            .edit()
            .remove(KEY_LOCKED_VAULT)
            .remove(KEY_REKEY_ROLLBACK_VAULT)
            .commit()

    private fun looksLikeLockedVaultJson(value: String): Boolean {
        val trimmed = value.trim()

        return trimmed.startsWith("{") &&
            trimmed.endsWith("}") &&
            trimmed.contains("\"version\"") &&
            trimmed.contains("\"public_key_b64\"") &&
            trimmed.contains("\"salt_b64\"") &&
            trimmed.contains("\"nonce_b64\"") &&
            trimmed.contains("\"ciphertext_b64\"") &&
            trimmed.contains("\"kdf\"")
    }

    private companion object {
        const val PREFERENCES_NAME = "scout_locked_vault"
        const val KEY_LOCKED_VAULT = "locked_vault_json"
        const val KEY_REKEY_ROLLBACK_VAULT = "rekey_rollback_vault_json"
    }
}
