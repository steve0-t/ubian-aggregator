package sk.cestou.platform

import android.os.Handler
import android.os.Looper
import android.webkit.CookieManager
import android.webkit.WebStorage
import java.util.concurrent.atomic.AtomicBoolean

// All transitions run on the UI thread. Interception threads read state only.
internal object ProviderSession {
    private var generation = 0
    @Volatile var state = "closed"
        private set
    @Volatile var lastError: String? = null
        private set

    fun beginOpen(): Boolean {
        check(Looper.myLooper() == Looper.getMainLooper())
        if (state != "closed") return false
        state = "opening"
        lastError = null
        return true
    }

    fun markOpen(): Boolean {
        if (state != "opening") return false
        state = "open"
        return true
    }

    fun error(code: String) { lastError = code }

    fun launchFailed() { state = "closed"; lastError = "launch_failed" }

    fun clean(beforeOpen: Boolean, done: (Boolean) -> Unit) {
        check(Looper.myLooper() == Looper.getMainLooper())
        state = "cleaning"
        val cleanupGeneration = ++generation
        val completed = AtomicBoolean(false)
        val handler = Handler(Looper.getMainLooper())
        fun complete(ok: Boolean) {
            if (cleanupGeneration != generation) return
            if (!completed.compareAndSet(false, true)) return
            state = if (ok) { if (beforeOpen) "opening" else "closed" } else "cleanup_failed"
            if (!ok) lastError = "cleanup_failed"
            done(ok)
        }
        handler.postDelayed({ complete(false) }, 15000)
        try {
            // WebStorage has no completion callback. Its disk/service-worker
            // behavior MUST be verified on device before persistent login ships.
            WebStorage.getInstance().deleteAllData()
            val cookies = CookieManager.getInstance()
            cookies.removeAllCookies {
                // Flush away deletions. No request may run while state=cleaning.
                try { cookies.flush(); complete(true) }
                catch (_: Exception) { complete(false) }
            }
        } catch (_: Exception) { complete(false) }
    }
}
