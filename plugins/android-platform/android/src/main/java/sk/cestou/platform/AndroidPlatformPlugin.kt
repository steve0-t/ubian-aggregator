package sk.cestou.platform

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.os.Build
import android.view.View
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

@InvokeArg
class OpenArgs {
    lateinit var destination: String
    lateinit var language: String
}

@InvokeArg
class AppearanceArgs {
    lateinit var theme: String
}

@TauriPlugin
class AndroidPlatformPlugin(private val activity: Activity) : Plugin(activity) {
    private val probeRunning = AtomicBoolean(false)
    private val worker = Executors.newSingleThreadExecutor()
    private val policy by lazy {
        RequestPolicy.parse(activity.assets.open("network-policy.json").bufferedReader().use { it.readText() })
    }

    @Command
    fun statusNative(invoke: Invoke) {
        try {
            invoke.resolve(JSObject().apply {
                put("android", true)
                put("policyVersion", policy.version)
                put("optionalRuleCount", policy.optionalRuleCount)
                put("providerState", ProviderSession.state)
                put("lastError", ProviderSession.lastError ?: org.json.JSONObject.NULL)
            })
        } catch (_: Exception) { invoke.reject("invalid_policy") }
    }

    @Command
    fun openProviderNative(invoke: Invoke) {
        var opening = false
        try {
            val args = invoke.parseArgs(OpenArgs::class.java)
            if (args.destination !in setOf("login", "support") || AppLanguage.parse(args.language) == null) {
                invoke.reject("invalid_destination")
                return
            }
            if (probeRunning.get() || !ProviderSession.beginOpen()) {
                invoke.reject("provider_busy")
                return
            }
            opening = true
            activity.startActivity(Intent(activity, ProviderActivity::class.java).apply {
                putExtra("destination", args.destination)
                putExtra("language", args.language)
            })
            invoke.resolve()
        } catch (_: Exception) {
            if (opening) ProviderSession.launchFailed()
            invoke.reject("provider_unavailable")
        }
    }

    @Command
    @Suppress("DEPRECATION")
    fun setAppearanceNative(invoke: Invoke) {
        try {
            val theme = invoke.parseArgs(AppearanceArgs::class.java).theme
            val color = when (theme) {
                "black" -> Color.BLACK
                "dark" -> Color.rgb(25, 26, 28)
                "light" -> Color.WHITE
                else -> { invoke.reject("invalid_theme"); return }
            }
            activity.runOnUiThread {
                try {
                    activity.window.decorView.setBackgroundColor(color)
                    activity.window.statusBarColor = color
                    activity.window.navigationBarColor = color
                    val light = View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR or View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR
                    activity.window.decorView.systemUiVisibility = if (theme == "light")
                        activity.window.decorView.systemUiVisibility or light
                    else activity.window.decorView.systemUiVisibility and light.inv()
                    if (Build.VERSION.SDK_INT >= 29) {
                        activity.window.isStatusBarContrastEnforced = false
                        activity.window.isNavigationBarContrastEnforced = false
                    }
                    invoke.resolve()
                } catch (_: Exception) { invoke.reject("appearance_unavailable") }
            }
        } catch (_: Exception) { invoke.reject("invalid_theme") }
    }

    @Command
    fun vaultStatusNative(invoke: Invoke) {
        worker.execute {
            try {
                val metadata = AccountVault.get(activity).availability()
                invoke.resolve(JSObject().apply {
                    put("availability", metadata.availability)
                    put("syntheticRecordPresent", metadata.syntheticRecordPresent)
                    put("realCredentialsEnabled", false)
                })
            } catch (_: Exception) { invoke.reject("secure_storage_unavailable") }
        }
    }

    @Command
    fun keystoreProbeNative(invoke: Invoke) {
        if (ProviderSession.state != "closed" || !probeRunning.compareAndSet(false, true)) {
            invoke.reject("provider_busy")
            return
        }
        worker.execute {
            try {
                val report = KeystoreProbe.run(activity.applicationContext)
                invoke.resolve(JSObject().apply {
                    put("roundTrip", report.roundTrip)
                    put("tamperRejected", report.tamperRejected)
                    put("uniqueNonces", report.uniqueNonces)
                    put("missingKeyRejected", report.missingKeyRejected)
                    put("hardwareBacked", report.hardwareBacked)
                })
            } catch (_: Exception) { invoke.reject("secure_storage_unavailable") }
            finally { probeRunning.set(false) }
        }
    }
}
