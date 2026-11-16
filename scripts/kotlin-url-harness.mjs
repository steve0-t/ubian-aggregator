import { readFileSync, writeFileSync } from "node:fs";

// Generate data literals from the SAME sanitized corpus as Rust and Android
// instrumentation. No production URL policy is reimplemented in this harness.
const cases = JSON.parse(
  readFileSync(
    new URL("../tests/fixtures/url-policy.json", import.meta.url),
    "utf8",
  ),
);
const literal = (value) => JSON.stringify(value).replaceAll("$", "\\$");
writeFileSync(
  process.argv[2],
  `package sk.cestou.platform

fun main() {
    val policy = RequestPolicy(1, listOf("www.ubian.sk"), emptyList())
${cases
  .map(
    (
      c,
    ) => `    check(policy.navigation(${literal(c.url)}) == Navigation.${c.navigation.toUpperCase()})
    check(policy.allowResource(${literal(c.url)}) == ${c.resource})`,
  )
  .join("\n")}
    val filtering = RequestPolicy(1, listOf("www.ubian.sk"), listOf(
        OptionalRule("www.ubian.sk", false, "/optional/", "analytics", "synthetic test"),
        OptionalRule("metrics.example.test", true, null, "analytics", "synthetic test")
    ))
    for (url in listOf("https://www.ubian.sk/optional/collect", "https://www.ubian.sk/public/%2e%2e/optional/collect", "https://www.ubian.sk/optional/.", "https://a.metrics.example.test/x")) check(!filtering.allowResource(url))
    for (url in listOf("https://www.ubian.sk/login", "https://metrics.example.test.attacker.test/x", "https://notmetrics.example.test/x")) check(filtering.allowResource(url))
    println("Kotlin policy: ${cases.length} shared URL cases and path/subdomain cases passed")
}
`,
);
