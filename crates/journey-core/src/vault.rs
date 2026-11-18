//! Internal preparation contracts. No secret payload or general read operation
//! is serializable or exported as a frontend command. Real account storage is gated.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VaultAvailability {
    Available,
    AuthenticationUnavailable,
    UnsupportedPlatform,
    KeyMissing,
    KeyInvalidated,
    InvalidRecord,
    IoFailure,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VaultFailure {
    AuthenticationUnavailable,
    AuthenticationCancelled,
    KeyMissing,
    KeyInvalidated,
    Tampered,
    UnsupportedVersion,
    InvalidRecord,
    IoFailure,
    Busy,
    NonceReused,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultStatus {
    pub availability: VaultAvailability,
    pub synthetic_record_present: bool,
    pub real_credentials_enabled: bool,
}

/// Native operations accept no caller-supplied secret and return status only.
/// Implementations serialize writes/deletion and fence late unlock callbacks.
pub trait SyntheticAccountVault {
    fn availability(&self) -> VaultStatus;
    fn save_or_update_synthetic(&mut self) -> Result<(), VaultFailure>;
    fn unlock_and_verify_synthetic(&mut self) -> Result<(), VaultFailure>;
    fn delete_synthetic(&mut self) -> Result<(), VaultFailure>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_metadata_never_advertises_an_account_or_secret() {
        let status = VaultStatus {
            availability: VaultAvailability::AuthenticationUnavailable,
            synthetic_record_present: false,
            real_credentials_enabled: false,
        };
        assert_eq!(
            serde_json::to_value(status).unwrap(),
            serde_json::json!({
                "availability": "authentication_unavailable", "syntheticRecordPresent": false,
                "realCredentialsEnabled": false
            })
        );
        assert!(serde_json::from_str::<VaultFailure>("\"unknown_failure\"").is_err());
    }
}
