package com.routalk.scoutoperator

internal object NativeBridge {
    private const val LIBRARY_NAME = "scout_operator_native"

    init {
        System.loadLibrary(LIBRARY_NAME)
    }

    external fun engineName(): String
    external fun bridgeStatus(): String
    external fun rpcCluster(): String
    external fun rpcEndpoint(): String
    external fun devnetBlockHeight(): String
    external fun signStageCDevnetProof(vaultJson: String, passphraseBytes: ByteArray): String
    external fun createLockedDevnetVault(passphraseBytes: ByteArray): String
    external fun verifyLockedDevnetPassphrase(vaultJson: String, passphraseBytes: ByteArray): String
    external fun exportLockedVaultRecoveryWords(vaultJson: String, passphraseBytes: ByteArray): String
    external fun verifyLockedVaultRecoveryWords(vaultJson: String, recoveryWords: String): String
    external fun lockedVaultDevnetAddress(vaultJson: String): String
    external fun lockedVaultDevnetBalance(vaultJson: String): String
    external fun lockedVaultDevnetHistory(vaultJson: String): String
    external fun createLockedVaultBackup(vaultJson: String): String
    external fun validateLockedVaultBackup(backupJson: String): String
    external fun extractValidatedLockedVaultBackup(backupJson: String): String
}
