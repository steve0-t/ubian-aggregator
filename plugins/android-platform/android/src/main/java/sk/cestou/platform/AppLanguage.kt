package sk.cestou.platform

internal enum class AppLanguage {
    SK, EN, UK;
    fun text(sk: String, en: String, uk: String): String = when (this) {
        SK -> sk
        EN -> en
        UK -> uk
    }
    companion object {
        fun parse(value: String?): AppLanguage? = when (value) {
            "sk" -> SK
            "en" -> EN
            "uk" -> UK
            else -> null
        }
    }
}
