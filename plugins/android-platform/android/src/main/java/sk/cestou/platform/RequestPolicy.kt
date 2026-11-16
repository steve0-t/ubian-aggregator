package sk.cestou.platform

import org.json.JSONObject
import java.net.URI
import java.util.Locale

internal enum class Navigation { PROVIDER, EXTERNAL, DENY }

internal data class OptionalRule(
    val host: String,
    val includeSubdomains: Boolean,
    val pathPrefix: String?,
    val category: String,
    val evidence: String,
)

internal class RequestPolicy(
    val version: Int,
    private val providerHosts: List<String>,
    private val optionalRules: List<OptionalRule>,
) {
    val optionalRuleCount: Int get() = optionalRules.size

    init {
        require(version == 1 && providerHosts.isNotEmpty())
        require(providerHosts.all(::validHost))
        require(optionalRules.all {
            validHost(it.host) && it.category.isNotEmpty() && it.evidence.isNotEmpty() &&
                (it.pathPrefix == null || (it.pathPrefix.startsWith('/') && it.pathPrefix.none { c -> c in "?#\\%" }))
        })
    }

    fun navigation(raw: String): Navigation {
        val uri = remoteUrl(raw) ?: return Navigation.DENY
        return if (uri.host.lowercase(Locale.ROOT) in providerHosts) Navigation.PROVIDER else Navigation.EXTERNAL
    }

    fun allowResource(raw: String): Boolean {
        val uri = remoteUrl(raw) ?: return false
        val host = uri.host.lowercase(Locale.ROOT)
        val path = canonicalPath(uri.rawPath)
        return optionalRules.none {
            (host == it.host || (it.includeSubdomains && host.endsWith(".${it.host}"))) &&
                (it.pathPrefix == null || path.startsWith(it.pathPrefix))
        }
    }

    companion object {
        private fun validHost(host: String): Boolean =
            host.isNotEmpty() && host == host.lowercase(Locale.ROOT) &&
                host.split('.').all { label ->
                    label.isNotEmpty() && !label.startsWith('-') && !label.endsWith('-') &&
                        label.all { it in 'a'..'z' || it in '0'..'9' || it == '-' }
                }

        fun remoteUrl(raw: String): URI? = try {
            val authority = raw.substringAfter("://", "").takeWhile { it !in "/?#" }
            if (raw.any { it.isWhitespace() || it.isISOControl() || it == '\\' } ||
                authority.isEmpty() || authority.endsWith(':') ||
                authority.any { it.code > 127 || it in "@%" }) {
                null
            } else {
                val uri = URI(raw)
                if (!uri.scheme.equals("https", true) || uri.host == null ||
                    !validHost(uri.host.lowercase(Locale.ROOT)) || uri.rawUserInfo != null ||
                    (uri.port != -1 && uri.port != 443)) null else uri
            }
        } catch (_: Exception) { null }

        private fun canonicalPath(raw: String?): String {
            val segments = mutableListOf<String>()
            val parts = (raw ?: "/").split('/')
            for ((index, segment) in parts.withIndex()) {
                when (segment.replace("%2e", ".", ignoreCase = true)) {
                    "." -> if (index == parts.lastIndex) segments.add("")
                    ".." -> {
                        if (segments.size > 1) segments.removeAt(segments.lastIndex)
                        if (index == parts.lastIndex) segments.add("")
                    }
                    else -> segments.add(segment)
                }
            }
            return segments.joinToString("/").ifEmpty { "/" }
        }

        fun parse(raw: String): RequestPolicy {
            val data = JSONObject(raw)
            val hosts = data.getJSONArray("providerHosts")
            val rules = data.getJSONArray("optionalRules")
            require(data.length() == 3)
            return RequestPolicy(
                data.getInt("version"),
                (0 until hosts.length()).map { hosts.getString(it) },
                (0 until rules.length()).map {
                    val rule = rules.getJSONObject(it)
                    require(rule.length() == 5)
                    OptionalRule(
                        rule.getString("host"), rule.getBoolean("includeSubdomains"),
                        if (rule.isNull("pathPrefix")) null else rule.getString("pathPrefix"),
                        rule.getString("category"), rule.getString("evidence"),
                    )
                },
            )
        }
    }
}
