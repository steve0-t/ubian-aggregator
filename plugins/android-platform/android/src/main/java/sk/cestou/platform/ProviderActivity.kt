package sk.cestou.platform

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.graphics.Bitmap
import android.net.Uri
import android.net.http.SslError
import android.os.Build
import android.os.Bundle
import android.os.Message
import android.view.View
import android.view.WindowInsets
import android.view.WindowManager
import android.webkit.CookieManager
import android.webkit.GeolocationPermissions
import android.webkit.PermissionRequest
import android.webkit.RenderProcessGoneDetail
import android.webkit.SafeBrowsingResponse
import android.webkit.ServiceWorkerClient
import android.webkit.ServiceWorkerController
import android.webkit.SslErrorHandler
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import android.window.OnBackInvokedCallback
import android.window.OnBackInvokedDispatcher

/** A platform WebView, never a Tauri WebView. No JavaScript/native bridge. */
class ProviderActivity : Activity() {
    private var provider: WebView? = null
    private var closing = false
    private var backCallback: OnBackInvokedCallback? = null
    private lateinit var policy: RequestPolicy
    private lateinit var feedback: TextView
    private lateinit var progress: ProgressBar
    private var english = false

    private fun text(sk: String, en: String) = if (english) en else sk

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        english = intent.getStringExtra("language") == "en"
        val destination = when (intent.getStringExtra("destination")) {
            "journeys" -> "https://www.ubian.sk/"
            "login" -> "https://www.ubian.sk/login"
            "support" -> "https://www.ubian.sk/pomoc-a-podpora"
            else -> null
        }
        // Never restore/replay a possibly state-changing WebView navigation.
        if (savedInstanceState != null || destination == null || ProviderSession.state != "opening") {
            finish()
            return
        }
        try {
            policy = RequestPolicy.parse(assets.open("network-policy.json").bufferedReader().use { it.readText() })
        } catch (_: Exception) { finish(); return }
        if (policy.navigation(destination) != Navigation.PROVIDER) { finish(); return }

        val root = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        val bar = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
        val close = Button(this).apply {
            this.text = text("Zavrieť", "Close")
            minHeight = (48 * resources.displayMetrics.density).toInt()
            setOnClickListener { finish() }
        }
        bar.addView(close)
        bar.addView(TextView(this).apply {
            this.text = text("Web Ubian · cookies tretích strán vypnuté", "Ubian website · third-party cookies off")
            setPadding(12, 12, 12, 12)
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        root.addView(bar)
        progress = ProgressBar(this, null, android.R.attr.progressBarStyleHorizontal)
        root.addView(progress)
        feedback = TextView(this).apply { setPadding(16, 8, 16, 8); accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE }
        root.addView(feedback)
        if (Build.VERSION.SDK_INT >= 30) {
            window.setDecorFitsSystemWindows(false)
            root.setOnApplyWindowInsetsListener { view, insets ->
                val safe = insets.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.ime())
                view.setPadding(safe.left, safe.top, safe.right, safe.bottom)
                insets
            }
        } else { root.fitsSystemWindows = true }
        setContentView(root)
        if (Build.VERSION.SDK_INT >= 33) {
            backCallback = OnBackInvokedCallback { back() }.also {
                onBackInvokedDispatcher.registerOnBackInvokedCallback(OnBackInvokedDispatcher.PRIORITY_DEFAULT, it)
            }
        }

        ProviderSession.clean(beforeOpen = true) { ok ->
            if (!ok || closing || isFinishing || isDestroyed) { finish(); return@clean }
            try { createProvider(root, destination) }
            catch (_: Exception) { showError("provider_unavailable"); finish() }
        }
    }

    @Suppress("SetJavaScriptEnabled")
    private fun createProvider(root: LinearLayout, destination: String) {
        val view = WebView(this)
        provider = view
        with(view.settings) {
            javaScriptEnabled = true
            domStorageEnabled = true
            allowFileAccess = false
            allowContentAccess = false
            mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
            javaScriptCanOpenWindowsAutomatically = false
            setSupportMultipleWindows(true)
            setGeolocationEnabled(false)
            cacheMode = WebSettings.LOAD_NO_CACHE
            safeBrowsingEnabled = true
            @Suppress("DEPRECATION")
            saveFormData = false
        }
        if (Build.VERSION.SDK_INT >= 26) view.importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
        CookieManager.getInstance().setAcceptThirdPartyCookies(view, false)
        view.clearCache(true)
        val filter = ResourceFilter(policy)
        // Do not attach a Tauri client, scripts or any JavascriptInterface here.
        view.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                if (closing || ProviderSession.state != "open") return true
                return when (policy.navigation(request.url.toString())) {
                    Navigation.PROVIDER -> false
                    Navigation.EXTERNAL -> {
                        if (request.isForMainFrame && request.hasGesture() && !request.isRedirect) external(request.url.toString())
                        else showError("external_unverified")
                        true
                    }
                    Navigation.DENY -> { showError("navigation_blocked"); true }
                }
            }

            override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse? = filter.intercept(request)

            override fun onPageStarted(view: WebView, url: String, favicon: Bitmap?) {
                if (closing || policy.navigation(url) != Navigation.PROVIDER) {
                    view.stopLoading()
                    showError("navigation_blocked")
                    return
                }
                if (RequestPolicy.remoteUrl(url)?.path?.startsWith("/login") == true ||
                    RequestPolicy.remoteUrl(url)?.path?.startsWith("/register") == true) {
                    // Remains secure for the rest of this account/login view.
                    window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
                }
                feedback.text = "www.ubian.sk"
            }

            override fun onReceivedSslError(view: WebView, handler: SslErrorHandler, error: SslError) {
                handler.cancel()
                showError("tls_error")
            }

            override fun onSafeBrowsingHit(view: WebView, request: WebResourceRequest, threatType: Int, callback: SafeBrowsingResponse) {
                callback.backToSafety(true)
                showError("unsafe_page")
            }

            override fun onReceivedError(view: WebView, request: WebResourceRequest, error: WebResourceError) {
                if (request.isForMainFrame) showError(if (error.errorCode == ERROR_HOST_LOOKUP || error.errorCode == ERROR_CONNECT) "offline" else "page_error")
            }

            override fun onReceivedHttpError(view: WebView, request: WebResourceRequest, response: WebResourceResponse) {
                if (request.isForMainFrame) showError("http_error")
            }

            override fun onFormResubmission(view: WebView, dontResend: Message, resend: Message) { dontResend.sendToTarget() }

            override fun onRenderProcessGone(view: WebView, detail: RenderProcessGoneDetail): Boolean {
                showError("renderer_unavailable")
                finish()
                return true
            }
        }
        view.webChromeClient = object : WebChromeClient() {
            override fun onProgressChanged(view: WebView, newProgress: Int) {
                progress.progress = newProgress
                progress.visibility = if (newProgress == 100) View.GONE else View.VISIBLE
            }
            override fun onPermissionRequest(request: PermissionRequest) { request.deny() }
            override fun onGeolocationPermissionsShowPrompt(origin: String, callback: GeolocationPermissions.Callback) { callback.invoke(origin, false, false) }
            override fun onShowFileChooser(view: WebView, callback: ValueCallback<Array<Uri>>, params: FileChooserParams): Boolean {
                callback.onReceiveValue(null)
                showError("uploads_unverified")
                return true
            }
            override fun onCreateWindow(view: WebView, dialog: Boolean, gesture: Boolean, result: Message): Boolean {
                showError("popup_unverified")
                return false
            }
        }
        view.setDownloadListener { _, _, _, _, _ -> showError("downloads_unverified") }
        val sw = ServiceWorkerController.getInstance()
        sw.serviceWorkerWebSettings.apply {
            allowFileAccess = false
            allowContentAccess = false
            cacheMode = WebSettings.LOAD_NO_CACHE
        }
        sw.setServiceWorkerClient(object : ServiceWorkerClient() {
            override fun shouldInterceptRequest(request: WebResourceRequest): WebResourceResponse? = filter.intercept(request)
        })
        root.addView(view, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f))
        if (ProviderSession.markOpen()) view.loadUrl(destination) else finish()
    }

    private fun external(raw: String) {
        val uri = RequestPolicy.remoteUrl(raw) ?: return
        if (raw.length > 8192) return
        AlertDialog.Builder(this)
            .setTitle(text("Otvoriť externý prehliadač?", "Open an external browser?"))
            .setMessage(text("Cieľ: ${uri.host}\nRelácia a ochrana ubian-aggregator sa neprenesú.", "Destination: ${uri.host}\nThe ubian-aggregator session and protection will not transfer."))
            .setNegativeButton(text("Zrušiť", "Cancel"), null)
            .setPositiveButton(text("Otvoriť", "Open")) { _, _ ->
                try {
                    startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(raw)).addCategory(Intent.CATEGORY_BROWSABLE))
                } catch (_: Exception) { showError("external_unavailable") }
            }.show()
    }

    private fun showError(code: String) {
        ProviderSession.error(code)
        if (!::feedback.isInitialized) return
        feedback.text = when (code) {
            "offline" -> text("Web nie je dostupný. Skontrolujte pripojenie.", "The website is unavailable. Check your connection.")
            "tls_error", "unsafe_page", "navigation_blocked" -> text("Navigácia bola zablokovaná z bezpečnostných dôvodov.", "Navigation was blocked for safety.")
            else -> text("Tento krok zatiaľ nie je overený alebo je nedostupný. Môžete sa vrátiť do ubian-aggregator.", "This step is unverified or unavailable. You can return to ubian-aggregator.")
        }
    }

    private fun back() {
        val view = provider
        if (!closing && view != null && view.canGoBack()) view.goBack() else finish()
    }

    @Deprecated("Legacy Back for Android 8–12")
    override fun onBackPressed() { back() }

    override fun onPause() { provider?.onPause(); super.onPause() }
    override fun onResume() { super.onResume(); provider?.onResume() }

    private fun destroyProvider() {
        provider?.let { view ->
            view.stopLoading()
            view.clearHistory()
            view.clearCache(true)
            (view.parent as? LinearLayout)?.removeView(view)
            view.destroy()
        }
        provider = null
    }

    override fun finish() {
        if (closing) return
        closing = true
        destroyProvider()
        ProviderSession.clean(beforeOpen = false) { super.finish() }
    }

    override fun onDestroy() {
        if (Build.VERSION.SDK_INT >= 33) backCallback?.let { onBackInvokedDispatcher.unregisterOnBackInvokedCallback(it) }
        destroyProvider()
        if (!closing) { closing = true; ProviderSession.clean(beforeOpen = false) {} }
        super.onDestroy()
    }
}
