package sk.cestou.platform

import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import java.io.ByteArrayInputStream

// A service-worker client can outlive the Activity. Retain immutable policy,
// never an Activity, WebView, URL history or credential-bearing request data.
internal class ResourceFilter(private val policy: RequestPolicy) {
    fun intercept(request: WebResourceRequest): WebResourceResponse? {
        val raw = request.url.toString()
        if (ProviderSession.state == "open" && policy.allowResource(raw) &&
            (!request.isForMainFrame || policy.navigation(raw) == Navigation.PROVIDER)) return null
        return WebResourceResponse("text/plain", "utf-8", 403, "Blocked", emptyMap(), ByteArrayInputStream(ByteArray(0)))
    }
}
