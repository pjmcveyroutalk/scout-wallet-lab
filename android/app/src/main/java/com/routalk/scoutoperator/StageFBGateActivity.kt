package com.routalk.scoutoperator

import android.app.Activity
import android.os.Bundle
import android.view.Gravity
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView

class StageFBGateActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)

        val padding = (24 * resources.displayMetrics.density).toInt()

        fun text(value: String, size: Float): TextView =
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

        root.addView(text("SCOUT STAGE F-B", 24f))
        root.addView(text("DEVNET ONLY", 18f))
        root.addView(text("MAINNET — DISABLED", 14f))
        root.addView(text("ARBITRARY SIGNING — DISABLED", 14f))
        root.addView(
            text(
                "Stage F-B is the separate operator gate that follows the completed Stage F-A presubmit proof. The ledger-write action is not armed in this build.",
                14f,
            ),
        )
        root.addView(
            text(
                "The final Stage F-B flow will preserve the fixed Scout Devnet proof candidate, fee ceiling, remaining-balance floor, one-signer identity check, exact signed-byte simulation, one-attempt rule, and read-only resolution after any uncertain delivery outcome.",
                14f,
            ),
        )

        val status =
            text(
                "IMPLEMENTATION GATE — NOT ARMED",
                17f,
            ).apply {
                gravity = Gravity.CENTER
                minHeight = padding * 4
            }
        root.addView(
            status,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        val close =
            Button(this).apply {
                text = "CLOSE"
                contentDescription = "Close the Scout Stage F-B gate"
                setOnClickListener { finish() }
            }
        fullWidth(close)
        root.addView(close)

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
