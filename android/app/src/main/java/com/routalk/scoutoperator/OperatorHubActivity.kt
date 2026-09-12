package com.routalk.scoutoperator

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.view.Gravity
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView

class OperatorHubActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)

        val padding = (24 * resources.displayMetrics.density).toInt()

        fun text(
            value: String,
            size: Float,
        ): TextView =
            TextView(this).apply {
                this.text = value
                textSize = size
                setPadding(0, padding / 2, 0, padding / 2)
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
                setPadding(padding, padding, padding, padding)
            }

        root.addView(text("SCOUT OPERATOR", 26f))
        root.addView(text("DEVNET ONLY", 18f))
        root.addView(
            text(
                "Choose an explicit operator gate. Mainnet and transaction submission remain disabled.",
                15f,
            ),
        )

        val recovery =
            Button(this).apply {
                text = "CREDENTIAL / RECOVERY"
                contentDescription =
                    "Open Scout credential verification, recovery words, and passphrase re-key controls"
                setOnClickListener {
                    startActivity(
                        Intent(
                            this@OperatorHubActivity,
                            CredentialRecoveryActivity::class.java,
                        ),
                    )
                }
            }
        fullWidth(recovery)
        root.addView(recovery)

        val wallet =
            Button(this).apply {
                text = "WALLET / READ-ONLY OPERATIONS"
                contentDescription =
                    "Open Scout wallet identity, balance, history, and encrypted backup controls"
                setOnClickListener {
                    startActivity(
                        Intent(
                            this@OperatorHubActivity,
                            MainActivity::class.java,
                        ),
                    )
                }
            }
        fullWidth(wallet)
        root.addView(wallet)

        val stageD =
            Button(this).apply {
                text = "STAGE D PHYSICAL DEVNET PROOF"
                contentDescription =
                    "Open the separate explicit Scout Stage D physical Devnet signature proof gate"
                setOnClickListener {
                    startActivity(
                        Intent(
                            this@OperatorHubActivity,
                            StageDProofActivity::class.java,
                        ),
                    )
                }
            }
        fullWidth(stageD)
        root.addView(stageD)

        root.addView(
            text(
                "Stage D signs only the fixed Scout Devnet proof operation after local passphrase entry and explicit authorization. It does not submit a transaction.",
                14f,
            ),
        )

        val stageE =
            Button(this).apply {
                text = "STAGE E DEVNET SIMULATION PREFLIGHT"
                contentDescription =
                    "Open the fixed Scout Stage E Devnet signing and simulation-only preflight gate"
                setOnClickListener {
                    startActivity(
                        Intent(
                            this@OperatorHubActivity,
                            StageEPreflightActivity::class.java,
                        ),
                    )
                }
            }
        fullWidth(stageE)
        root.addView(stageE)

        root.addView(
            text(
                "Stage E preflight signs only a fixed Memo proof candidate, checks the Devnet fee, and simulates it with signature verification. Transaction submission remains disabled.",
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
    }
}
