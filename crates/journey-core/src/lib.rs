//! Policy shared by the coordinator and the Android request-filter implementation.
//! No HTTP adapter, credentials, or provider session is owned by this crate.

pub mod timetable;
pub mod vault;

use serde::{Deserialize, Serialize};
use url::Url;

pub const POLICY_JSON: &str = include_str!("../../../policy/network-policy.json");

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Destination {
    Login,
    Support,
}

impl Destination {
    pub fn url(self) -> &'static str {
        match self {
            Self::Login => "https://www.ubian.sk/login",
            Self::Support => "https://www.ubian.sk/pomoc-a-podpora",
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkPolicy {
    pub version: u32,
    provider_hosts: Vec<String>,
    optional_rules: Vec<OptionalRule>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OptionalRule {
    host: String,
    include_subdomains: bool,
    path_prefix: Option<String>,
    category: String,
    evidence: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Navigation {
    Provider,
    External,
    Deny,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResourceDecision<'a> {
    Allow,
    Deny,
    BlockOptional(&'a str),
}

fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.is_ascii()
        && host == host.to_ascii_lowercase()
        && !host.ends_with('.')
        && host.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

/// Reject parser repair, ambiguous authorities, userinfo (including empty userinfo),
/// non-HTTPS, non-default ports, and DNS suffix tricks before using a parsed URL.
/// Query/fragment values are never returned to diagnostics.
pub fn remote_url(raw: &str) -> Option<Url> {
    if raw
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
    {
        return None;
    }
    let (scheme, rest) = raw.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.is_empty()
        || authority.ends_with(':')
        || !authority.is_ascii()
        || authority.contains(['@', '%'])
    {
        return None;
    }
    let parsed = Url::parse(raw).ok()?;
    if parsed.scheme() != "https"
        || parsed.port_or_known_default() != Some(443)
        || !valid_host(parsed.host_str()?)
    {
        return None;
    }
    Some(parsed)
}

impl NetworkPolicy {
    pub fn parse(json: &str) -> Result<Self, &'static str> {
        let policy: Self = serde_json::from_str(json).map_err(|_| "invalid_policy")?;
        if policy.version != 1
            || policy.provider_hosts.is_empty()
            || policy.provider_hosts.iter().any(|host| !valid_host(host))
            || policy.optional_rules.iter().any(|rule| {
                !valid_host(&rule.host)
                    || rule.category.is_empty()
                    || rule.evidence.is_empty()
                    || rule
                        .path_prefix
                        .as_ref()
                        .is_some_and(|p| !p.starts_with('/') || p.contains(['?', '#', '\\', '%']))
            })
        {
            return Err("invalid_policy");
        }
        Ok(policy)
    }

    pub fn navigation(&self, raw: &str) -> Navigation {
        let Some(url) = remote_url(raw) else {
            return Navigation::Deny;
        };
        if self
            .provider_hosts
            .iter()
            .any(|h| Some(h.as_str()) == url.host_str())
        {
            Navigation::Provider
        } else {
            Navigation::External
        }
    }

    /// Apply to subresources as well as first-party paths. A functional origin
    /// never bypasses a path-specific optional rule.
    pub fn resource(&self, raw: &str) -> ResourceDecision<'_> {
        let Some(url) = remote_url(raw) else {
            return ResourceDecision::Deny;
        };
        let host = url.host_str().unwrap_or_default();
        for rule in &self.optional_rules {
            let host_matches = host == rule.host
                || (rule.include_subdomains && host.ends_with(&format!(".{}", rule.host)));
            if host_matches
                && rule
                    .path_prefix
                    .as_ref()
                    .is_none_or(|prefix| url.path().starts_with(prefix))
            {
                return ResourceDecision::BlockOptional(&rule.category);
            }
        }
        ResourceDecision::Allow
    }

    pub fn optional_rule_count(&self) -> usize {
        self.optional_rules.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Case {
        url: String,
        navigation: Navigation,
        resource: bool,
    }

    #[test]
    fn shared_malicious_url_corpus() {
        let policy = NetworkPolicy::parse(POLICY_JSON).unwrap();
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("../../../tests/fixtures/url-policy.json")).unwrap();
        for case in cases {
            assert_eq!(
                policy.navigation(&case.url),
                case.navigation,
                "{}",
                case.url
            );
            assert_eq!(
                policy.resource(&case.url) == ResourceDecision::Allow,
                case.resource,
                "{}",
                case.url
            );
        }
    }

    #[test]
    fn path_rules_apply_to_first_party_and_subdomains_only_when_requested() {
        // Synthetic rules are never shipped as claims about Ubian traffic.
        let policy = NetworkPolicy::parse(r#"{
          "version":1,"providerHosts":["www.ubian.sk"],"optionalRules":[
            {"host":"www.ubian.sk","includeSubdomains":false,"pathPrefix":"/optional/","category":"analytics","evidence":"synthetic test"},
            {"host":"metrics.example.test","includeSubdomains":true,"pathPrefix":null,"category":"analytics","evidence":"synthetic test"}
          ]}"#).unwrap();
        for url in [
            "https://www.ubian.sk/optional/collect",
            "https://metrics.example.test/x",
            "https://a.metrics.example.test/x",
            "https://www.ubian.sk/public/%2e%2e/optional/collect",
            "https://www.ubian.sk/optional/.",
        ] {
            assert_eq!(
                policy.resource(url),
                ResourceDecision::BlockOptional("analytics")
            );
        }
        for url in [
            "https://www.ubian.sk/login",
            "https://www.ubian.sk/optional-login",
            "https://metrics.example.test.attacker.test/x",
            "https://notmetrics.example.test/x",
        ] {
            assert_eq!(policy.resource(url), ResourceDecision::Allow);
        }
    }

    #[test]
    fn reject_unknown_policy_versions_and_unreviewed_rules() {
        for json in [
            r#"{"version":2,"providerHosts":["www.ubian.sk"],"optionalRules":[]}"#,
            r#"{"version":1,"providerHosts":["*.ubian.sk"],"optionalRules":[]}"#,
            r#"{"version":1,"providerHosts":[],"optionalRules":[]}"#,
            r#"{"version":1,"providerHosts":["www.ubian.sk"],"optionalRules":[{"host":"metrics.example.test","includeSubdomains":false,"pathPrefix":null,"category":"analytics","evidence":""}]}"#,
        ] {
            assert!(NetworkPolicy::parse(json).is_err());
        }
    }

    #[test]
    fn all_entry_points_use_the_provider_policy() {
        let policy = NetworkPolicy::parse(POLICY_JSON).unwrap();
        assert_eq!(policy.optional_rule_count(), 0);
        for destination in [Destination::Login, Destination::Support] {
            assert_eq!(policy.navigation(destination.url()), Navigation::Provider);
        }
    }
}
