<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import {
    accountCancel,
    accountForget,
    accountLogin,
    accountLogout,
    accountRefresh,
    accountSaveSession,
    accountStatus,
    accountUnlockSession,
    isNative,
    setAccountProtection,
    type AccountSnapshot,
  } from "./api/platform";
  import { accountError, type AccountError } from "./account-state";
  import { messages, locales, type Language } from "./i18n";
  import { SearchRequest } from "./timetable-state";

  let {
    language,
    canOpenProvider,
    openLogin,
  }: {
    language: Language;
    canOpenProvider: boolean;
    openLogin: () => void;
  } = $props();
  const t = $derived(messages[language]);
  const native = isNative();
  let email = $state("");
  let password = $state("");
  let account = $state<AccountSnapshot | null>(null);
  let operation = $state<
    | "status"
    | "login"
    | "refresh"
    | "logout"
    | "forget"
    | "save"
    | "unlock"
    | null
  >(null);
  let error = $state<AccountError | null>(null);
  let notice = $state<
    "accountLogoutDone" | "accountLogoutUnconfirmed" | "accountForgotten" | null
  >(null);
  let fresh = $state(false);
  let summaryHeading = $state<HTMLHeadingElement>();
  const requests = new SearchRequest(accountCancel);
  const signedIn = $derived(account?.state === "signed_in");
  const hasSession = $derived(signedIn || account?.state === "cached");
  const canSave = $derived(
    signedIn && account?.storage.availability === "available",
  );
  const canUnlock = $derived(
    account?.storage.recordPresent &&
      account.storage.availability === "available",
  );
  const stateMessage = $derived(
    operation === "login" || account?.state === "signing_in"
      ? "accountSigningIn"
      : account?.state === "expired"
        ? "accountExpired"
        : hasSession
          ? fresh
            ? "accountSignedIn"
            : "accountUnverified"
          : "accountSignedOut",
  );
  const lastUpdated = $derived(
    account?.lastUpdated
      ? new Intl.DateTimeFormat(locales[language], {
          dateStyle: "medium",
          timeStyle: "short",
          timeZone: "Europe/Bratislava",
        }).format(new Date(account.lastUpdated))
      : null,
  );

  function clearAccount(state: "signed_out" | "expired" = "signed_out") {
    email = "";
    password = "";
    fresh = false;
    if (account)
      account = { ...account, state, profile: null, lastUpdated: null };
  }
  function apply(view: AccountSnapshot, verified: boolean) {
    account =
      view.state === "expired"
        ? { ...view, profile: null, lastUpdated: null }
        : view;
    fresh = verified && view.state === "signed_in";
  }
  async function run(
    kind: NonNullable<typeof operation>,
    action: (requestId: string) => Promise<AccountSnapshot>,
    verified = false,
  ) {
    const ticket = requests.begin();
    operation = kind;
    error = null;
    notice = null;
    if (kind !== "save") fresh = false;
    try {
      const view = await action(requests.requestId(ticket));
      if (requests.current(ticket)) {
        apply(view, verified);
        if ((kind === "login" || kind === "unlock") && view.profile) {
          await tick();
          if (!requests.current(ticket)) return false;
          summaryHeading?.focus();
        }
        return true;
      }
    } catch (failure) {
      if (requests.current(ticket)) {
        error = accountError(failure);
        fresh = false;
        if (failure === "session_expired" || failure === "not_signed_in")
          clearAccount("expired");
        if (kind === "logout" || kind === "forget") {
          // Cleanup can clear the in-memory account while failing to remove its
          // encrypted record. Read actual status and leave a retry available.
          let view: AccountSnapshot;
          try {
            view = await accountStatus();
          } catch {
            view = {
              state: "signed_out",
              profile: null,
              lastUpdated: null,
              storage: { availability: "io_failure", recordPresent: true },
            };
          }
          if (requests.current(ticket)) apply(view, false);
        }
      }
    } finally {
      if (requests.current(ticket)) {
        operation = null;
        requests.finish(ticket);
      }
    }
    return false;
  }
  function submit(event: SubmitEvent) {
    event.preventDefault();
    const submittedEmail = email.trim();
    const submittedPassword = password;
    // Clear both controls before awaiting IPC. Neither field is persisted by UI.
    email = "";
    password = "";
    if (!submittedEmail || !submittedPassword) {
      requests.cancel();
      operation = null;
      error = "accountInvalidInput";
      return;
    }
    clearAccount();
    void run(
      "login",
      (id) => accountLogin(submittedEmail, submittedPassword, false, id),
      true,
    );
  }
  function cancel() {
    requests.cancel();
    operation = null;
    password = "";
    fresh = false;
    error = "accountCancelled";
  }
  async function logout() {
    clearAccount();
    let serverConfirmed = false;
    const completed = await run("logout", async (id) => {
      const result = await accountLogout(id);
      serverConfirmed = result.serverConfirmed;
      return result.account;
    });
    if (completed)
      notice = serverConfirmed
        ? "accountLogoutDone"
        : "accountLogoutUnconfirmed";
  }
  async function forget() {
    clearAccount();
    const completed = await run("forget", (id) => accountForget(id));
    if (completed) notice = "accountForgotten";
  }
  function registrationOrRecovery() {
    email = "";
    password = "";
    openLogin();
  }
  onMount(() => {
    if (native) {
      void setAccountProtection(true).catch(() => {});
      void run("status", () => accountStatus());
    }
  });
  onDestroy(() => {
    requests.cancel();
    email = "";
    password = "";
    if (native) void setAccountProtection(false).catch(() => {});
  });
</script>

<section class="native-account panel" aria-labelledby="native-account-title">
  <h2 id="native-account-title">{t.accountTitle}</h2>
  <p class="account-state" role="status">
    {operation === "save"
      ? t.accountSaving
      : operation === "unlock"
        ? t.accountUnlocking
        : operation && operation !== "login"
          ? t.busy
          : t[stateMessage]}
  </p>
  {#if error}<p class="account-message account-error" role="alert">
      {t[error]}
    </p>{/if}
  {#if notice}<p class="account-message" role="status">{t[notice]}</p>{/if}
  {#if !native}<p class="muted">{t.nativeUnavailable}</p>{/if}
  {#if !signedIn}
    <form
      onsubmit={submit}
      aria-busy={operation === "login"}
      autocomplete="off"
    >
      <label for="account-email"
        >{t.email}<input
          id="account-email"
          type="email"
          bind:value={email}
          maxlength="254"
          required
          autocomplete="off"
          autocapitalize="none"
          spellcheck="false"
          disabled={!!operation}
        /></label
      >
      <label for="account-password"
        >{t.password}<input
          id="account-password"
          type="password"
          bind:value={password}
          maxlength="4096"
          required
          autocomplete="off"
          disabled={!!operation}
        /></label
      >
      <p class="muted">{t.nativeLoginHint}</p>
      <button class="primary" type="submit" disabled={!native || !!operation}
        >{operation === "login" ? t.accountSigningIn : t.nativeLogin}</button
      >
    </form>
  {/if}
  {#if account?.profile}
    <section class="account-summary" aria-labelledby="account-summary-title">
      <h3 id="account-summary-title" tabindex="-1" bind:this={summaryHeading}>
        {t.accountSummary}
      </h3>
      {#if !fresh}<p class="account-cache-label" role="status">
          {t.accountCached}
        </p>{/if}
      <dl>
        {#if account.profile.displayName}<dt>{t.accountName}</dt>
          <dd>{account.profile.displayName}</dd>{/if}
        <dt>{t.email}</dt>
        <dd>{account.profile.email}</dd>
        {#if lastUpdated}<dt>{t.accountLastUpdated}</dt>
          <dd>{lastUpdated}</dd>{/if}
      </dl>
    </section>
  {/if}
  {#if hasSession}
    <div class="account-actions">
      <button
        class="secondary"
        type="button"
        disabled={!!operation}
        onclick={() => void run("refresh", (id) => accountRefresh(id), true)}
        >{operation === "refresh" ? t.busy : t.accountRefresh}</button
      >
      <button
        class="secondary"
        type="button"
        disabled={!!operation}
        onclick={() => void logout()}>{t.accountLogout}</button
      >
    </div>
  {/if}
  {#if operation === "login" || operation === "refresh" || operation === "unlock"}<button
      class="secondary account-cancel"
      type="button"
      onclick={cancel}>{t.cancel}</button
    >{/if}
  <section class="account-storage" aria-labelledby="account-storage-title">
    <h3 id="account-storage-title">{t.rememberSession}</h3>
    <p class="muted">{t.accountRememberHint}</p>
    {#if !account}<p class="muted">{t.busy}</p>
    {:else if account.storage.availability === "io_failure"}
      <p class="muted">{t.accountStorageUnknown}</p>
    {:else if account.storage.recordPresent && (account.storage.availability === "key_missing" || account.storage.availability === "key_invalidated" || account.storage.availability === "invalid_record")}
      <p class="muted">{t.accountSessionInvalid}</p>
    {:else if account.storage.recordPresent}
      <p>{t.accountSessionSaved}</p>
      {#if !signedIn}<p class="muted">{t.accountStorageLocked}</p>{/if}
    {:else}<p class="muted">
        {#if hasSession}{t.accountSessionMemory}{/if}
        {t.accountRememberOff}
      </p>{/if}
    {#if account && account.storage.availability !== "available" && account.storage.availability !== "io_failure"}<p
        class="muted"
      >
        {t.accountStorageUnavailable}
      </p>{/if}
    <div class="account-actions">
      {#if signedIn}<button
          class="secondary"
          type="button"
          disabled={!canSave || !!operation}
          onclick={() =>
            void run("save", (id) => accountSaveSession(id, language), fresh)}
          >{operation === "save"
            ? t.accountSaving
            : t.accountSaveSession}</button
        >{/if}
      {#if account?.storage.recordPresent && !hasSession}<button
          class="secondary"
          type="button"
          disabled={!canUnlock || !!operation}
          onclick={() =>
            void run(
              "unlock",
              (id) => accountUnlockSession(id, language),
              true,
            )}
          >{operation === "unlock"
            ? t.accountUnlocking
            : t.accountUnlockSession}</button
        >{/if}
    </div>
  </section>
  <div class="account-provider-links">
    <button
      class="secondary"
      type="button"
      disabled={!canOpenProvider || !!operation}
      onclick={registrationOrRecovery}>{t.accountRegistrationRecovery}</button
    >
    <p class="muted">{t.accountWebsiteHint}</p>
  </div>
  {#if hasSession || account?.storage.recordPresent}
    <section class="account-removal" aria-label={t.accountForget}>
      <button
        class="secondary"
        type="button"
        disabled={!!operation}
        aria-describedby="account-forget-hint"
        onclick={() => void forget()}>{t.accountForget}</button
      >
      <p class="muted" id="account-forget-hint">{t.accountForgetHint}</p>
    </section>
  {/if}
</section>
