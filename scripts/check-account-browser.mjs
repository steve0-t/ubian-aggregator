// Account UI regression using synthetic IPC only. No real credentials, cookies
// or authenticated traffic are consumed or produced by this harness.
import assert from "node:assert/strict";
const { chromium } = await import(
  process.env.PLAYWRIGHT_MODULE ?? "playwright"
);
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_EXECUTABLE,
  headless: true,
  args: ["--no-sandbox", "--disable-dev-shm-usage"],
});
try {
  const page = await browser.newPage({ viewport: { width: 360, height: 800 } });
  page.setDefaultTimeout(10000);
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.addInitScript(() => {
    const signedOut = () => ({
      state: "signed_out",
      profile: null,
      lastUpdated: null,
      storage: { availability: "available", recordPresent: false },
    });
    const state = (window.__accountTest = {
      calls: [],
      account: signedOut(),
      loginMode: "success",
      refreshMode: "success",
      unlockMode: "success",
      logoutMode: "success",
      forgetMode: "success",
      pending: null,
    });
    const authenticated = (accountState = "signed_in") => ({
      state: accountState,
      profile: {
        email: "native-ui-test@example.invalid",
        displayName: "<img src=x onerror=alert(1)>",
      },
      lastUpdated: "2026-11-18T08:20:00+01:00",
      storage: state.account.storage,
    });
    window.__TAURI__ = {
      core: {
        invoke: async (command, args = {}) => {
          const name = command.split("|")[1];
          state.calls.push({ name, args });
          switch (name) {
            case "status":
              return {
                android: true,
                policyVersion: 1,
                optionalRuleCount: 0,
                providerState: "closed",
                lastError: null,
              };
            case "set_appearance":
            case "open_provider":
            case "account_cancel":
            case "account_protection":
              return;
            case "account_status":
              return structuredClone(state.account);
            case "account_login":
              if (state.loginMode === "pending")
                return new Promise((resolve) => (state.pending = resolve));
              if (state.loginMode !== "success") throw state.loginMode;
              state.account = authenticated();
              return structuredClone(state.account);
            case "account_refresh":
              if (state.refreshMode === "session_expired") {
                state.account = {
                  ...signedOut(),
                  state: "expired",
                  storage: state.account.storage,
                };
                throw "session_expired";
              }
              if (state.refreshMode !== "success") throw state.refreshMode;
              state.account = authenticated();
              return structuredClone(state.account);
            case "account_save_session":
              state.account.storage.recordPresent = true;
              return structuredClone(state.account);
            case "account_unlock_session":
              if (state.unlockMode === "authentication_cancelled")
                throw state.unlockMode;
              state.account = authenticated(
                state.unlockMode === "cached" ? "cached" : "signed_in",
              );
              return structuredClone(state.account);
            case "account_logout":
              state.account = signedOut();
              if (state.logoutMode === "cleanup_failed") {
                state.account.storage = {
                  availability: "io_failure",
                  recordPresent: true,
                };
                throw "cleanup_failed";
              }
              if (state.logoutMode === "pending")
                return new Promise((resolve) => (state.pending = resolve));
              return {
                account: structuredClone(state.account),
                serverConfirmed: false,
              };
            case "account_forget":
              if (state.forgetMode === "cleanup_failed") {
                state.account = {
                  ...signedOut(),
                  storage: { availability: "io_failure", recordPresent: true },
                };
                throw "cleanup_failed";
              }
              state.account = signedOut();
              return structuredClone(state.account);
            default:
              throw "unexpected_test_command";
          }
        },
      },
    };
  });
  await page.goto(process.env.UBIAN_UI_URL ?? "http://127.0.0.1:1422", {
    waitUntil: "domcontentloaded",
  });
  const account = page.locator(".native-account");
  const nav = page.locator(".mobile-navigation");
  const openAccount = async () => {
    await nav.getByRole("button", { name: "Účet", exact: true }).click();
    await account.locator("#account-email").waitFor();
    await page.waitForFunction(
      () => !document.querySelector("#account-email")?.disabled,
    );
  };
  const signIn = async () => {
    await account
      .locator("#account-email")
      .fill("native-ui-test@example.invalid");
    await account.locator("#account-password").fill("synthetic-ui-password");
    await account
      .getByRole("button", { name: "Prihlásiť sa do Ubian", exact: true })
      .click();
  };
  const refresh = account.getByRole("button", {
    name: "Overiť reláciu a obnoviť účet",
    exact: true,
  });
  await openAccount();
  assert.ok(
    await page.evaluate(() =>
      window.__accountTest.calls.some(
        (call) => call.name === "account_protection" && call.args.enabled,
      ),
    ),
  );
  assert.match(await account.innerText(), /Nie ste prihlásení/);
  await signIn();
  await account.getByText("Relácia overená v Ubian", { exact: true }).waitFor();
  assert.equal(await account.locator("img").count(), 0);
  assert.match(
    await account.locator(".account-summary").innerText(),
    /<img src=x onerror=alert\(1\)>/,
  );
  const login = await page.evaluate(() =>
    window.__accountTest.calls.find((call) => call.name === "account_login"),
  );
  assert.deepEqual(login.args.request, {
    email: "native-ui-test@example.invalid",
    password: "synthetic-ui-password",
    rememberSession: false,
  });
  assert.match(login.args.requestId, /^timetable-/);
  await account
    .getByRole("button", { name: "Uložiť aktuálnu reláciu", exact: true })
    .click();
  await account
    .getByText("Relácia je uložená v tomto zariadení.", { exact: true })
    .waitFor();
  const save = await page.evaluate(() =>
    window.__accountTest.calls.find(
      (call) => call.name === "account_save_session",
    ),
  );
  assert.equal(save.args.language, "sk");
  assert.match(save.args.requestId, /^timetable-/);

  await page.evaluate(() => (window.__accountTest.refreshMode = "offline"));
  await refresh.click();
  await account.getByRole("alert").waitFor();
  assert.match(
    await account.getByRole("alert").innerText(),
    /Nie ste pripojení/,
  );
  assert.match(
    await account.locator(".account-cache-label").innerText(),
    /Posledný známy/,
  );
  await page.evaluate(
    () => (window.__accountTest.refreshMode = "session_expired"),
  );
  await refresh.click();
  await account.locator("#account-password").waitFor();
  assert.equal(await account.locator(".account-summary").count(), 0);

  await page.evaluate(
    () => (window.__accountTest.unlockMode = "authentication_cancelled"),
  );
  await account
    .getByRole("button", { name: "Odomknúť uloženú reláciu", exact: true })
    .click();
  await account
    .getByText("Overenie zariadenia bolo zrušené.", { exact: true })
    .waitFor();
  assert.equal(await account.locator(".account-summary").count(), 0);

  await page.evaluate(() => (window.__accountTest.unlockMode = "cached"));
  await account
    .getByRole("button", { name: "Odomknúť uloženú reláciu", exact: true })
    .click();
  await account.locator(".account-cache-label").waitFor();
  assert.equal(await account.locator("#account-email").count(), 1);
  assert.match(
    await account.locator(".account-state").innerText(),
    /nie je aktuálne overená/,
  );
  await page.evaluate(() => (window.__accountTest.refreshMode = "success"));
  await refresh.click();
  await account.getByText("Relácia overená v Ubian", { exact: true }).waitFor();
  await account
    .getByRole("button", { name: "Odhlásiť sa", exact: true })
    .click();
  await account
    .getByText(
      "Miestna relácia bola odstránená. Odhlásenie na serveri sa nepodarilo potvrdiť.",
      { exact: true },
    )
    .waitFor();
  assert.equal(await account.locator(".account-summary").count(), 0);

  // Cancellation must prevent an IPC completion from repopulating the screen.
  await page.evaluate(() => (window.__accountTest.loginMode = "pending"));
  await signIn();
  assert.equal(await account.locator("#account-password").inputValue(), "");
  assert.equal(await account.locator("#account-email").inputValue(), "");

  await account.getByRole("button", { name: "Zrušiť", exact: true }).click();
  await page.evaluate(() => {
    const state = window.__accountTest;
    state.pending({
      state: "signed_in",
      profile: {
        email: "late@example.invalid",
        displayName: "Late completion",
      },
      lastUpdated: "2026-11-18T08:20:00+01:00",
      storage: { availability: "available", recordPresent: false },
    });
  });
  await page.waitForTimeout(50);
  assert.equal(await account.locator(".account-summary").count(), 0);
  assert.match(await account.getByRole("alert").innerText(), /bola zrušená/);
  assert.ok(
    await page.evaluate(() =>
      window.__accountTest.calls.some(
        (call) => call.name === "account_cancel" && call.args.requestId,
      ),
    ),
  );

  await page.evaluate(
    () => (window.__accountTest.loginMode = "invalid_credentials"),
  );
  await signIn();
  await account
    .getByText("E-mail alebo heslo nie sú správne.", { exact: true })
    .waitFor();
  assert.equal(await account.locator("#account-password").inputValue(), "");
  await page.evaluate(
    () =>
      (window.__accountTest.loginMode =
        "<script>private provider error</script>"),
  );
  await signIn();
  await account
    .getByText("Operácia účtu sa nepodarila. Skúste to znova.", { exact: true })
    .waitFor();
  assert.doesNotMatch(await account.innerText(), /private provider error/);

  // Leaving the form destroys its sensitive input and cancels any in-flight work.
  await account.locator("#account-email").fill("abandoned@example.invalid");
  await account.locator("#account-password").fill("abandoned-ui-password");
  await nav.getByRole("button", { name: "Cesty", exact: true }).click();
  await openAccount();
  assert.equal(await account.locator("#account-password").inputValue(), "");
  assert.equal(await account.locator("#account-email").inputValue(), "");

  await account.locator("#account-email").fill("handoff@example.invalid");
  await account.locator("#account-password").fill("handoff-ui-password");
  await account
    .getByRole("button", {
      name: "Registrácia alebo zabudnuté heslo",
      exact: true,
    })
    .click();
  assert.equal(await account.locator("#account-password").inputValue(), "");
  assert.equal(await account.locator("#account-email").inputValue(), "");
  assert.ok(
    await page.evaluate(() =>
      window.__accountTest.calls.some(
        (call) =>
          call.name === "open_provider" && call.args.destination === "login",
      ),
    ),
  );

  // Unavailable encrypted storage must not prevent a memory-only login.
  await page.evaluate(() => {
    window.__accountTest.account.storage.availability = "unsupported_platform";
    window.__accountTest.loginMode = "success";
  });
  await nav.getByRole("button", { name: "Cesty", exact: true }).click();
  await openAccount();
  await signIn();
  await account.getByText("Relácia overená v Ubian", { exact: true }).waitFor();
  assert.equal(
    await account
      .getByRole("button", { name: "Uložiť aktuálnu reláciu", exact: true })
      .isDisabled(),
    true,
  );
  await account
    .getByRole("button", { name: "Zabudnúť účet", exact: true })
    .click();
  await account
    .getByText("Miestne údaje účtu boli odstránené.", { exact: true })
    .waitFor();

  // A delayed sign-out completion cannot restore a previous component's data.
  await signIn();
  await account.getByText("Relácia overená v Ubian", { exact: true }).waitFor();
  await page.evaluate(() => (window.__accountTest.logoutMode = "pending"));
  await account
    .getByRole("button", { name: "Odhlásiť sa", exact: true })
    .click();
  assert.equal(await account.locator(".account-summary").count(), 0);
  await nav.getByRole("button", { name: "Cesty", exact: true }).click();
  await openAccount();
  await page.evaluate(() => {
    window.__accountTest.pending({
      account: {
        state: "signed_in",
        profile: {
          email: "late-logout@example.invalid",
          displayName: "Late logout",
        },
        lastUpdated: "2026-11-18T08:20:00+01:00",
        storage: { availability: "available", recordPresent: false },
      },
      serverConfirmed: true,
    });
  });
  await page.waitForTimeout(50);
  assert.equal(await account.locator(".account-summary").count(), 0);
  assert.equal(
    await account.getByText("Ste odhlásení.", { exact: true }).count(),
    0,
  );
  assert.ok(
    await page.evaluate(() =>
      window.__accountTest.calls.some(
        (call) => call.name === "account_protection" && !call.args.enabled,
      ),
    ),
  );

  // Deletion failure must show the cleared local account and keep retrying
  // deletion available, without claiming that unknown saved storage is empty.
  await page.evaluate(
    () => (window.__accountTest.logoutMode = "cleanup_failed"),
  );
  await signIn();
  await account.getByText("Relácia overená v Ubian", { exact: true }).waitFor();
  const beforeCleanupStatus = await page.evaluate(
    () =>
      window.__accountTest.calls.filter(
        (call) => call.name === "account_status",
      ).length,
  );
  await account
    .getByRole("button", { name: "Odhlásiť sa", exact: true })
    .click();
  await account
    .getByText(
      "Miestne odhlásenie sa dokončilo, ale uložené údaje môžu zostať v zariadení. Skúste znova možnosť Zabudnúť účet.",
      { exact: true },
    )
    .waitFor();
  assert.equal(await account.locator(".account-summary").count(), 0);
  assert.match(
    await account.locator(".account-storage").innerText(),
    /Stav uložených údajov sa nepodarilo prečítať/,
  );
  assert.doesNotMatch(
    await account.locator(".account-storage").innerText(),
    /Relácia zostáva iba v pamäti/,
  );
  assert.ok(
    await page.evaluate(
      (count) =>
        window.__accountTest.calls.filter(
          (call) => call.name === "account_status",
        ).length > count,
      beforeCleanupStatus,
    ),
  );
  await page.evaluate(
    () => (window.__accountTest.forgetMode = "cleanup_failed"),
  );
  await account
    .getByRole("button", { name: "Zabudnúť účet", exact: true })
    .click();
  await account.getByRole("alert").waitFor();
  assert.equal(
    await account
      .getByRole("button", { name: "Zabudnúť účet", exact: true })
      .isEnabled(),
    true,
  );
  await page.evaluate(() => (window.__accountTest.forgetMode = "success"));
  await account
    .getByRole("button", { name: "Zabudnúť účet", exact: true })
    .click();
  await account
    .getByText("Miestne údaje účtu boli odstránené.", { exact: true })
    .waitFor();

  await page.setViewportSize({ width: 320, height: 800 });
  await page.evaluate(() => (document.documentElement.style.fontSize = "200%"));
  assert.ok(
    await account.evaluate(
      (element) => element.scrollWidth <= element.clientWidth + 2,
    ),
  );
  const stored = await page.evaluate(() => JSON.stringify({ ...localStorage }));
  for (const secret of [
    "native-ui-test@example.invalid",
    "synthetic-ui-password",
    "abandoned-ui-password",
    "handoff-ui-password",
  ])
    assert.equal(stored.includes(secret), false);
  assert.deepEqual(errors, []);
  console.log(
    "Account UI: native login/status, escaped profile, explicit session save, offline/expired/cached states, cancellation and late completion, forgotten input, storage fallback and no persisted credentials passed using test-only IPC.",
  );
} finally {
  await browser.close();
}
