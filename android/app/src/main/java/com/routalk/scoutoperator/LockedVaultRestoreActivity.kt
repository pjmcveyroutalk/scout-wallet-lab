package com.routalk.scoutoperator

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.os.Bundle
import android.view.Gravity
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast

internal class LockedVaultRestoreActivity : Activity() {
    private var statusView: TextView? = null
    private var selectBackupButton: Button? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)

        val padding =
            (24 * resources.displayMetrics.density).toInt()

        fun text(
            value: String,
            size: Float,
        ): TextView =
            TextView(this).apply {
                this.text = value
                textSize = size
                setPadding(
                    0,
                    padding / 2,
                    0,
                    padding / 2,
                )
            }

        fun fullWidth(button: Button) {
            button.layoutParams =
                ViewGroup.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.WRAP_CONTENT,
                )
        }

        val root =
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_HORIZONTAL
                setPadding(
                    padding,
                    padding,
                    padding,
                    padding,
                )
            }

        root.addView(
            text(
                "SCOUT RECOVERY / RESTORE",
                24f,
            ),
        )

        root.addView(
            text(
                "DEVNET ONLY",
                16f,
            ),
        )

        root.addView(
            text(
                "Encrypted locked-vault recovery",
                14f,
            ),
        )

        root.addView(
            text(
                "No passphrase, seed, private key, signing, " +
                    "transaction submission, or Mainnet access " +
                    "is used by this restore path.",
                16f,
            ),
        )

        val storageState =
            inspectStorage()

        root.addView(
            text(
                "Current wallet storage",
                14f,
            ),
        )

        root.addView(
            text(
                storageState.status,
                16f,
            ),
        )

        val status =
            text(
                when {
                    storageState.occupied &&
                        storageState.lockedVaultJson != null ->
                        "READY — OCCUPIED-STORAGE REFUSAL TEST\n\n" +
                            "Scout must refuse restore and leave the " +
                            "current encrypted vault unchanged."

                    storageState.occupied ->
                        "RESTORE BLOCKED — STORAGE IS OCCUPIED " +
                            "BUT CURRENT VAULT CANNOT BE VERIFIED"

                    else ->
                        "READY — EMPTY STORAGE\n\n" +
                            "A verified encrypted Devnet backup may " +
                            "be restored after explicit confirmation."
                },
                18f,
            ).apply {
                gravity = Gravity.CENTER
                minHeight = padding * 5
            }

        statusView = status

        root.addView(
            status,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val selectBackup =
            Button(this).apply {
                text =
                    if (storageState.occupied) {
                        "SELECT BACKUP + VERIFY REFUSAL"
                    } else {
                        "SELECT ENCRYPTED BACKUP"
                    }

                isEnabled =
                    !storageState.occupied ||
                        storageState.lockedVaultJson != null

                contentDescription =
                    if (storageState.occupied) {
                        "Select encrypted Scout Devnet backup " +
                            "and verify occupied storage refuses restore"
                    } else {
                        "Select encrypted Scout Devnet backup " +
                            "for controlled restore"
                    }
            }

        selectBackupButton = selectBackup

        fullWidth(selectBackup)
        root.addView(selectBackup)

        val close =
            Button(this).apply {
                text = "CLOSE"
                contentDescription =
                    "Close Scout recovery and restore"
            }

        fullWidth(close)
        root.addView(close)

        root.addView(
            text(
                "MAINNET — DISABLED",
                14f,
            ),
        )

        root.addView(
            text(
                "TRANSACTION SUBMISSION — DISABLED",
                14f,
            ),
        )

        root.addView(
            text(
                "SIGNING — LOCKED",
                14f,
            ),
        )

        val scrollView =
            ScrollView(this).apply {
                addView(
                    root,
                    ViewGroup.LayoutParams(
                        ViewGroup.LayoutParams.MATCH_PARENT,
                        ViewGroup.LayoutParams.WRAP_CONTENT,
                    ),
                )
            }

        setContentView(scrollView)

        selectBackup.setOnClickListener {
            selectBackup.isEnabled = false
            status.text =
                "CHOOSE ENCRYPTED SCOUT DEVNET BACKUP..."

            try {
                startActivityForResult(
                    LockedVaultBackupFileIO.createImportIntent(),
                    REQUEST_IMPORT_RESTORE_BACKUP,
                )
            } catch (error: Throwable) {
                status.text =
                    "RESTORE IMPORT FAILED — " +
                        error.javaClass.simpleName

                selectBackup.isEnabled = true
            }
        }

        close.setOnClickListener {
            finish()
        }
    }

    @Deprecated(
        "Deprecated in Android API; retained for current Activity result boundary.",
    )
    override fun onActivityResult(
        requestCode: Int,
        resultCode: Int,
        data: Intent?,
    ) {
        super.onActivityResult(
            requestCode,
            resultCode,
            data,
        )

        if (requestCode != REQUEST_IMPORT_RESTORE_BACKUP) {
            return
        }

        if (resultCode != RESULT_OK) {
            statusView?.text =
                "RESTORE CANCELED"

            selectBackupButton?.isEnabled = true
            return
        }

        val source =
            data?.data
                ?: run {
                    statusView?.text =
                        "RESTORE FAILED — BACKUP SOURCE MISSING"

                    selectBackupButton?.isEnabled = true
                    return
                }

        statusView?.text =
            "READING ENCRYPTED DEVNET BACKUP..."

        Thread {
            val fileResult =
                try {
                    LockedVaultBackupFileIO.readBackup(
                        contentResolver = contentResolver,
                        source = source,
                    )
                } catch (error: Throwable) {
                    LockedVaultBackupFileIO.FileResult(
                        success = false,
                        content = null,
                        status =
                            "BACKUP IMPORT FAILED — " +
                                error.javaClass.simpleName,
                    )
                }

            if (!fileResult.success) {
                runOnUiThread {
                    statusView?.text =
                        fileResult.status

                    selectBackupButton?.isEnabled = true
                }

                return@Thread
            }

            val backupJson =
                fileResult.content
                    ?: run {
                        runOnUiThread {
                            statusView?.text =
                                "RESTORE FAILED — BACKUP PACKAGE MISSING"

                            selectBackupButton?.isEnabled = true
                        }

                        return@Thread
                    }

            val storageBefore =
                inspectStorage()

            if (storageBefore.occupied) {
                runOccupiedStorageRefusal(
                    backupJson = backupJson,
                    storageBefore = storageBefore,
                )

                return@Thread
            }

            validateBeforeEmptyStorageRestore(
                backupJson = backupJson,
            )
        }.start()
    }

    private fun runOccupiedStorageRefusal(
        backupJson: String,
        storageBefore: StorageState,
    ) {
        val vaultBefore =
            storageBefore.lockedVaultJson
                ?: run {
                    runOnUiThread {
                        statusView?.text =
                            "RESTORE SAFETY TEST BLOCKED — " +
                                "CURRENT VAULT UNREADABLE"

                        selectBackupButton?.isEnabled = true
                    }

                    return
                }

        val addressBefore =
            verifiedAddress(
                lockedVaultJson = vaultBefore,
            )

        val restoreResult =
            try {
                LockedVaultRestoreCoordinator.restoreIntoEmptyStorage(
                    context = this,
                    backupJson = backupJson,
                )
            } catch (error: Throwable) {
                LockedVaultRestoreCoordinator.RestoreResult(
                    success = false,
                    publicAddress = null,
                    status =
                        "RESTORE TEST FAILED — " +
                            error.javaClass.simpleName,
                )
            }

        val storageAfter =
            inspectStorage()

        val vaultAfter =
            storageAfter.lockedVaultJson

        val addressAfter =
            vaultAfter?.let {
                verifiedAddress(
                    lockedVaultJson = it,
                )
            }

        val refusalConfirmed =
            !restoreResult.success &&
                restoreResult.status ==
                "RESTORE BLOCKED — EXISTING ENCRYPTED VAULT PRESENT"

        val storageUnchanged =
            storageAfter.occupied &&
                vaultAfter == vaultBefore

        val identityUnchanged =
            !addressBefore.isNullOrBlank() &&
                addressAfter == addressBefore

        val pass =
            refusalConfirmed &&
                storageUnchanged &&
                identityUnchanged

        runOnUiThread {
            statusView?.text =
                if (pass) {
                    buildString {
                        append("RESTORE SAFETY PROOF")
                        append("\n\n")
                        append("PASS — OCCUPIED STORAGE REFUSED")
                        append("\n")
                        append("NO RESTORE WRITE PERFORMED")
                        append("\n")
                        append("ENCRYPTED VAULT UNCHANGED")
                        append("\n")
                        append("DEVNET IDENTITY UNCHANGED")
                        append("\n\n")
                        append("Address: ")
                        append(addressAfter)
                    }
                } else {
                    buildString {
                        append("RESTORE SAFETY PROOF")
                        append("\n\n")
                        append("BLOCKED / FAILED")
                        append("\n\n")
                        append(restoreResult.status)
                        append("\n\n")
                        append(
                            if (storageUnchanged) {
                                "Encrypted vault unchanged"
                            } else {
                                "CRITICAL — STORAGE CHANGE DETECTED"
                            },
                        )
                        append("\n")
                        append(
                            if (identityUnchanged) {
                                "Devnet identity unchanged"
                            } else {
                                "IDENTITY NOT VERIFIED UNCHANGED"
                            },
                        )
                    }
                }

            Toast.makeText(
                this,
                if (pass) {
                    "RESTORE REFUSAL PASS — CURRENT VAULT UNCHANGED"
                } else {
                    "RESTORE SAFETY CHECK BLOCKED / FAILED"
                },
                Toast.LENGTH_LONG,
            ).show()

            selectBackupButton?.isEnabled = true
        }
    }

    private fun validateBeforeEmptyStorageRestore(
        backupJson: String,
    ) {
        val validationResult =
            try {
                NativeBridge.validateLockedVaultBackup(
                    backupJson,
                )
            } catch (error: Throwable) {
                "backup-validation-failed:" +
                    error.javaClass.simpleName
            }

        if (!validationResult.startsWith("ok:")) {
            runOnUiThread {
                statusView?.text =
                    "RESTORE BLOCKED — BACKUP VALIDATION FAILED"

                selectBackupButton?.isEnabled = true
            }

            return
        }

        val publicAddress =
            validationResult
                .removePrefix("ok:")
                .trim()

        if (publicAddress.isBlank()) {
            runOnUiThread {
                statusView?.text =
                    "RESTORE BLOCKED — BACKUP IDENTITY MISSING"

                selectBackupButton?.isEnabled = true
            }

            return
        }

        val storageRecheck =
            inspectStorage()

        if (storageRecheck.occupied) {
            runOnUiThread {
                statusView?.text =
                    "RESTORE BLOCKED — STORAGE BECAME OCCUPIED"

                selectBackupButton?.isEnabled = true
            }

            return
        }

        runOnUiThread {
            showEmptyStorageRestoreConfirmation(
                backupJson = backupJson,
                publicAddress = publicAddress,
            )
        }
    }

    private fun showEmptyStorageRestoreConfirmation(
        backupJson: String,
        publicAddress: String,
    ) {
        if (
            isFinishing ||
            isDestroyed
        ) {
            return
        }

        statusView?.text =
            "VERIFIED ENCRYPTED BACKUP — " +
                "RESTORE CONFIRMATION REQUIRED"

        AlertDialog.Builder(this)
            .setTitle(
                "Restore encrypted Devnet vault?",
            )
            .setMessage(
                buildString {
                    append(
                        "Scout verified the encrypted backup package.",
                    )
                    append("\n\n")
                    append("Devnet address:")
                    append("\n")
                    append(publicAddress)
                    append("\n\n")
                    append(
                        "Current Scout vault storage is empty.",
                    )
                    append("\n\n")
                    append(
                        "Continuing writes only the encrypted " +
                            "LockedVault contained in this backup.",
                    )
                    append("\n\n")
                    append(
                        "No passphrase, seed, private key, signing, " +
                            "transaction submission, or Mainnet " +
                            "operation is performed.",
                    )
                },
            )
            .setCancelable(true)
            .setNegativeButton(
                "CANCEL",
            ) { dialog, _ ->
                dialog.dismiss()

                statusView?.text =
                    "RESTORE CANCELED — NO WRITE PERFORMED"

                selectBackupButton?.isEnabled = true
            }
            .setPositiveButton(
                "RESTORE ENCRYPTED VAULT",
            ) { dialog, _ ->
                dialog.dismiss()

                performEmptyStorageRestore(
                    backupJson = backupJson,
                    expectedAddress = publicAddress,
                )
            }
            .show()
    }

    private fun performEmptyStorageRestore(
        backupJson: String,
        expectedAddress: String,
    ) {
        selectBackupButton?.isEnabled = false
        statusView?.text =
            "RESTORING VERIFIED ENCRYPTED DEVNET VAULT..."

        Thread {
            val restoreResult =
                try {
                    LockedVaultRestoreCoordinator.restoreIntoEmptyStorage(
                        context = this,
                        backupJson = backupJson,
                    )
                } catch (error: Throwable) {
                    LockedVaultRestoreCoordinator.RestoreResult(
                        success = false,
                        publicAddress = null,
                        status =
                            "RESTORE FAILED — " +
                                error.javaClass.simpleName,
                    )
                }

            val storageAfter =
                inspectStorage()

            val restoredVaultJson =
                storageAfter.lockedVaultJson

            val restoredAddress =
                restoredVaultJson?.let {
                    verifiedAddress(
                        lockedVaultJson = it,
                    )
                }

            val verified =
                restoreResult.success &&
                    storageAfter.occupied &&
                    !restoredVaultJson.isNullOrBlank() &&
                    !restoredAddress.isNullOrBlank() &&
                    restoredAddress == expectedAddress &&
                    restoreResult.publicAddress == expectedAddress

            runOnUiThread {
                statusView?.text =
                    if (verified) {
                        buildString {
                            append("RESTORE")
                            append("\n\n")
                            append("PASS — VERIFIED")
                            append("\n")
                            append(
                                "ENCRYPTED DEVNET VAULT RESTORED",
                            )
                            append("\n")
                            append("IDENTITY VERIFIED")
                            append("\n\n")
                            append("Address: ")
                            append(restoredAddress)
                        }
                    } else {
                        buildString {
                            append("RESTORE")
                            append("\n\n")
                            append("BLOCKED / FAILED")
                            append("\n\n")
                            append(restoreResult.status)
                        }
                    }

                Toast.makeText(
                    this,
                    if (verified) {
                        "ENCRYPTED DEVNET VAULT RESTORE PASS"
                    } else {
                        "RESTORE BLOCKED / FAILED"
                    },
                    Toast.LENGTH_LONG,
                ).show()

                selectBackupButton?.isEnabled =
                    !verified
            }
        }.start()
    }

    private fun inspectStorage(): StorageState =
        try {
            val vaultStore =
                LockedVaultStore(this)

            StorageState(
                occupied = vaultStore.hasVault(),
                lockedVaultJson = vaultStore.loadVault(),
                status =
                    when {
                        vaultStore.hasVault() &&
                            !vaultStore.loadVault().isNullOrBlank() ->
                            "ENCRYPTED VAULT PRESENT"

                        vaultStore.hasVault() ->
                            "STORAGE ENTRY PRESENT — VAULT UNREADABLE"

                        else ->
                            "EMPTY — NO ENCRYPTED VAULT STORED"
                    },
            )
        } catch (error: Throwable) {
            StorageState(
                occupied = true,
                lockedVaultJson = null,
                status =
                    "STORAGE STATE UNAVAILABLE — " +
                        error.javaClass.simpleName +
                        " — RESTORE FAIL-CLOSED",
            )
        }

    private fun verifiedAddress(
        lockedVaultJson: String,
    ): String? {
        if (lockedVaultJson.isBlank()) {
            return null
        }

        return try {
            val result =
                NativeBridge.lockedVaultDevnetAddress(
                    lockedVaultJson,
                )

            if (!result.startsWith("ok:")) {
                null
            } else {
                result
                    .removePrefix("ok:")
                    .trim()
                    .takeIf { it.isNotBlank() }
            }
        } catch (_: Throwable) {
            null
        }
    }

    override fun onDestroy() {
        statusView = null
        selectBackupButton = null

        super.onDestroy()
    }

    private data class StorageState(
        val occupied: Boolean,
        val lockedVaultJson: String?,
        val status: String,
    )

    private companion object {
        const val REQUEST_IMPORT_RESTORE_BACKUP =
            4201
    }
}
