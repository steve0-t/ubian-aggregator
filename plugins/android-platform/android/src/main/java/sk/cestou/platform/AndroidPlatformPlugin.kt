package sk.cestou.platform

import android.app.Activity
import android.content.Intent
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
            if (args.destination !in setOf("journeys", "login", "support") || args.language !in setOf("sk", "en")) {
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
