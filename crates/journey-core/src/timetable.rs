//! Anonymous source discovery only. Production traffic is gated on verified
//! integration permission; an HTTP 200 or robots.txt is not an access grant.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Sk,
    En,
    Uk,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimetableError {
    SourceUnavailable,
    InvalidQuery,
    ProviderChanged,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StopQuery {
    pub query: String,
}

impl StopQuery {
    pub fn validate(&self) -> Result<&str, TimetableError> {
        let trimmed = self.query.trim();
        if !(2..=100).contains(&trimmed.chars().count()) || self.query.chars().any(char::is_control)
        {
            return Err(TimetableError::InvalidQuery);
        }
        Ok(trimmed)
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Stop {
    pub id: u32,
    pub name: String,
}

pub fn lookup_stops(query: StopQuery) -> Result<Vec<Stop>, TimetableError> {
    query.validate()?;
    // Deliberately no client, arbitrary URL, cookie jar, or fixture fallback.
    Err(TimetableError::SourceUnavailable)
}

/// Narrow parsing spike for the actually observed anonymous response. Provider
/// metadata is discarded; names/identifiers are preserved, never interpreted as HTML.
pub fn parse_stops(bytes: &[u8]) -> Result<Vec<Stop>, TimetableError> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Response {
        status: String,
        input_phrase: String,
        results: Vec<Item>,
    }
    #[derive(Deserialize)]
    struct Item {
        #[serde(rename = "type")]
        kind: String,
        id: u32,
        name: String,
    }
    if bytes.len() > 256 * 1024 {
        return Err(TimetableError::ProviderChanged);
    }
    let response: Response =
        serde_json::from_slice(bytes).map_err(|_| TimetableError::ProviderChanged)?;
    if response.status != "ok"
        || response.input_phrase.chars().count() > 100
        || response.results.len() > 100
    {
        return Err(TimetableError::ProviderChanged);
    }
    response
        .results
        .into_iter()
        .map(|item| {
            if item.kind != "stop"
                || item.id == 0
                || item.name.trim().is_empty()
                || item.name.chars().count() > 256
                || item.name.chars().any(char::is_control)
            {
                return Err(TimetableError::ProviderChanged);
            }
            Ok(Stop {
                id: item.id,
                name: item.name,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observed_sanitized_stop_response() {
        let stops = parse_stops(include_bytes!(
            "../../../tests/fixtures/provider/stops.json"
        ))
        .expect("observed response");
        assert_eq!(stops.len(), 3);
        assert_eq!(stops[1].id, 950001485);
        assert_eq!(stops[1].name, "Bratislava hlavná stanica, žel.st.");
    }

    #[test]
    fn empty_data_is_distinct_from_a_schema_or_provider_error() {
        assert!(
            parse_stops(br#"{"status":"ok","inputPhrase":"xx","results":[]}"#)
                .unwrap()
                .is_empty()
        );
        for body in [
            r#"{"status":"error","inputPhrase":"xx","results":[]}"#,
            r#"{"status":"ok","results":[]}"#,
            r#"{"status":"ok","inputPhrase":"xx","results":[{"type":"stop","id":0,"name":"X"}]}"#,
            r#"{"status":"ok","inputPhrase":"xx","results":[{"type":"stop","id":1,"name":""}]}"#,
            r#"{"status":"ok","inputPhrase":"xx","results":[{"type":"station","id":1,"name":"Changed schema"}]}"#,
            "<html>Challenge</html>",
        ] {
            assert_eq!(
                parse_stops(body.as_bytes()),
                Err(TimetableError::ProviderChanged)
            );
        }
        assert_eq!(
            parse_stops(&vec![b' '; 256 * 1024 + 1]),
            Err(TimetableError::ProviderChanged)
        );
    }

    #[test]
    fn validate_input_and_fail_closed_without_network_or_mock_results() {
        for query in ["", "a", "ab\n", "ab\u{0000}", &"a".repeat(101)] {
            assert_eq!(
                lookup_stops(StopQuery {
                    query: query.into()
                }),
                Err(TimetableError::InvalidQuery)
            );
        }
        assert_eq!(
            lookup_stops(StopQuery {
                query: "Київ".into()
            }),
            Err(TimetableError::SourceUnavailable)
        );
    }

    #[test]
    fn ukrainian_crosses_the_native_language_boundary() {
        assert_eq!(
            serde_json::from_str::<Language>("\"uk\"").unwrap(),
            Language::Uk
        );
        assert!(serde_json::from_str::<Language>("\"xx\"").is_err());
    }
}
