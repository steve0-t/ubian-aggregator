export type AccountError =
  | "accountInvalidCredentials"
  | "accountInvalidInput"
  | "accountOffline"
  | "accountTimeout"
  | "accountRateLimited"
  | "accountChallenge"
  | "accountSchemaChanged"
  | "accountExpired"
  | "accountCancelled"
  | "accountUnavailable"
  | "accountStorageUnavailable"
  | "accountUnlockCancelled"
  | "accountSessionInvalid"
  | "accountSessionStoreFailed"
  | "accountCleanupFailed"
  | "accountActionFailed"
  | "nativeUnavailable";

export function accountError(error: unknown): AccountError {
  switch (error) {
    case "invalid_credentials":
      return "accountInvalidCredentials";
    case "invalid_input":
      return "accountInvalidInput";
    case "offline":
      return "accountOffline";
    case "timeout":
      return "accountTimeout";
    case "rate_limited":
      return "accountRateLimited";
    case "challenge_required":
      return "accountChallenge";
    case "provider_changed":
      return "accountSchemaChanged";
    case "session_expired":
    case "not_signed_in":
      return "accountExpired";
    case "cancelled":
      return "accountCancelled";
    case "provider_unavailable":
      return "accountUnavailable";
    case "authentication_unavailable":
    case "secure_storage_unavailable":
    case "unsupported_platform":
      return "accountStorageUnavailable";
    case "authentication_cancelled":
      return "accountUnlockCancelled";
    case "persistence_invalid":
    case "tampered":
    case "unsupported_version":
    case "key_missing":
    case "key_invalidated":
    case "invalid_record":
      return "accountSessionInvalid";
    case "io_failure":
    case "nonce_reused":
      return "accountSessionStoreFailed";
    case "cleanup_failed":
      return "accountCleanupFailed";
    case "android_required":
      return "nativeUnavailable";
    default:
      return "accountActionFailed";
  }
}
