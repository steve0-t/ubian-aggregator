import test from "node:test";
import assert from "node:assert/strict";
import { accountError } from "../src/lib/account-state.ts";
import { messages } from "../src/lib/i18n/index.ts";

test("account failures map to complete local messages without provider text", () => {
  const failures = {
    invalid_credentials: "accountInvalidCredentials",
    invalid_input: "accountInvalidInput",
    offline: "accountOffline",
    timeout: "accountTimeout",
    rate_limited: "accountRateLimited",
    challenge_required: "accountChallenge",
    provider_changed: "accountSchemaChanged",
    session_expired: "accountExpired",
    not_signed_in: "accountExpired",
    cancelled: "accountCancelled",
    provider_unavailable: "accountUnavailable",
    authentication_unavailable: "accountStorageUnavailable",
    secure_storage_unavailable: "accountStorageUnavailable",
    unsupported_platform: "accountStorageUnavailable",
    authentication_cancelled: "accountUnlockCancelled",
    persistence_invalid: "accountSessionInvalid",
    tampered: "accountSessionInvalid",
    unsupported_version: "accountSessionInvalid",
    key_missing: "accountSessionInvalid",
    key_invalidated: "accountSessionInvalid",
    invalid_record: "accountSessionInvalid",
    io_failure: "accountSessionStoreFailed",
    nonce_reused: "accountSessionStoreFailed",
    cleanup_failed: "accountCleanupFailed",
    android_required: "nativeUnavailable",
  };
  for (const [failure, message] of Object.entries(failures)) {
    assert.equal(accountError(failure), message);
    for (const dictionary of Object.values(messages))
      assert.ok(dictionary[message].trim());
  }
  assert.equal(
    accountError("<script>provider response</script>"),
    "accountActionFailed",
  );
  assert.equal(
    accountError({ message: "provider response" }),
    "accountActionFailed",
  );
});
