package sk.cestou.platform

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.os.Build
import android.view.View
import android.webkit.WebView
import androidx.appcompat.app.AppCompatActivity
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

@InvokeArg
class SessionStoreArgs {
    lateinit var snapshot: String
    lateinit var language: String
    var generation: Long = -1
}

@InvokeArg
class SessionUnlockArgs {
    lateinit var language: String
    var generation: Long = -1
}

@InvokeArg
class SessionGenerationArgs { var generation: Long = -1 }

@InvokeArg
class AccountProtectionArgs { var enabled: Boolean = false }

@TauriPlugin
class AndroidPlatformPlugin(initialActivity: Activity) : Plugin(initialActivity) {
    @Volatile private var activity: Activity = initialActivity
    private val probeRunning = AtomicBoolean(false)
    private val worker = Executors.newSingleThreadExecutor()
    @Volatile private var sessionVault = NativeSessionStorage.attach(initialActivity)
    private val policy by lazy {
        RequestPolicy.parse(activity.assets.open("network-policy.json").bufferedReader().use { it.readText() })
    }

    @Suppress("DEPRECATION")
    override fun load(webView: WebView) {
        super.load(webView)
        // The bundled login form follows the same persistence policy as the
        // provider view. Remembering a native session is an explicit vault action.
        webView.settings.saveFormData = false
        if (Build.VERSION.SDK_INT >= 26) {
            webView.importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
        }
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

    // These mobile bridge operations are called internally by the Rust account
    // coordinator. No frontend command exposes a raw snapshot or secret read.
    @Command
    fun sessionStorageStatusNative(invoke: Invoke) {
        worker.execute {
            try {
                val status = sessionVault.status()
                invoke.resolve(JSObject().apply {
                    put("availability", status.availability)
                    put("recordPresent", status.recordPresent)
                })
            } catch (error: Exception) { rejectSession(invoke, error) }
        }
    }

    @Command
    fun sessionStoreNative(invoke: Invoke) {
        worker.execute {
            var bytes: ByteArray? = null
            try {
                val args = invoke.parseArgs(SessionStoreArgs::class.java)
                val language = AppLanguage.parse(args.language) ?: throw VaultException(VaultFailure.INVALID_RECORD)
                if (args.snapshot.length > SessionRecord.MAX_PLAINTEXT) throw VaultException(VaultFailure.INVALID_RECORD)
                org.json.JSONObject(args.snapshot) // Only an opaque JSON object from Rust is accepted.
                bytes = args.snapshot.toByteArray(Charsets.UTF_8)
                args.snapshot = ""
                val consumed = bytes
                bytes = null
                sessionVault.store(consumed, language, args.generation) { result ->
                    result.fold({ invoke.resolve() }, { rejectSession(invoke, it) })
                }
            } catch (error: Exception) { rejectSession(invoke, error) }
            finally { bytes?.fill(0) }
        }
    }

    @Command
    fun sessionRestoreNative(invoke: Invoke) {
        worker.execute {
            try {
                val args = invoke.parseArgs(SessionUnlockArgs::class.java)
                val language = AppLanguage.parse(args.language) ?: throw VaultException(VaultFailure.INVALID_RECORD)
                sessionVault.restore(language, args.generation) { result ->
                    result.fold({ bytes ->
                        try {
                            val snapshot = Charsets.UTF_8.newDecoder()
                                .onMalformedInput(java.nio.charset.CodingErrorAction.REPORT)
                                .onUnmappableCharacter(java.nio.charset.CodingErrorAction.REPORT)
                                .decode(java.nio.ByteBuffer.wrap(bytes)).toString()
                            org.json.JSONObject(snapshot)
                            invoke.resolve(JSObject().apply { put("snapshot", snapshot) })
                        } catch (_: Exception) { invoke.reject("invalid_record") }
                        finally { bytes.fill(0) }
                    }, { rejectSession(invoke, it) })
                }
            } catch (error: Exception) { rejectSession(invoke, error) }
        }
    }

    @Command
    fun sessionDeleteNative(invoke: Invoke) {
        worker.execute {
            try {
                val args = invoke.parseArgs(SessionGenerationArgs::class.java)
                sessionVault.delete(args.generation).fold({ invoke.resolve() }, { rejectSession(invoke, it) })
            }
            catch (error: Exception) { rejectSession(invoke, error) }
        }
    }

    @Command
    fun sessionCancelNative(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(SessionGenerationArgs::class.java)
            sessionVault.cancel(args.generation)
            invoke.resolve()
        } catch (error: Exception) { rejectSession(invoke, error) }
    }

    @Command
    fun setAccountProtectionNative(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(AccountProtectionArgs::class.java)
            activity.runOnUiThread {
                try { AccountProtection.account(activity, args.enabled); invoke.resolve() }
                catch (_: Exception) { invoke.reject("io_failure") }
            }
        } catch (_: Exception) { invoke.reject("invalid_input") }
    }

    override fun onResume(activity: AppCompatActivity) {
        if (this.activity !== activity) {
            this.activity = activity
            sessionVault = NativeSessionStorage.attach(activity)
        }
        sessionVault.resume()
        super.onResume(activity)
    }

    override fun onStop(activity: AppCompatActivity) {
        if (this.activity === activity) sessionVault.pause()
        super.onStop(activity)
    }

    override fun onDestroy(activity: AppCompatActivity) {
        if (this.activity === activity) sessionVault.pause()
        super.onDestroy(activity)
    }

    private fun rejectSession(invoke: Invoke, error: Throwable) {
        // Never attach the exception: Tauri's overload would log it and might
        // expose native data. Java/JVM String copies cannot be fully erased.
        invoke.reject((error as? VaultException)?.failure?.name?.lowercase() ?: "io_failure")
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
