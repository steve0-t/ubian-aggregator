<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { name as appName } from "../package.json";
  import Icon from "./lib/Icon.svelte";
  import Timetables from "./lib/Timetables.svelte";
  import Account from "./lib/Account.svelte";
  import { messages, type Language } from "./lib/i18n";
  import {
    ACCENT_PRESETS,
    DEFAULT_ACCENT,
    applyAppearance,
    isAccent,
    savePreferences,
    type Preferences,
    type Theme,
  } from "./lib/preferences";
  import {
    isNative,
    openProvider,
    platformStatus,
    probeKeystore,
    saveDeviceTheme,
    setSystemAppearance,
    type Destination,
    type PlatformStatus,
    type ProbeReport,
  } from "./lib/api/platform";

  let {
    initialPreferences,
    initialThemeSaved,
  }: { initialPreferences: Preferences; initialThemeSaved: boolean } = $props();
  const startingPreferences = untrack(() => initialPreferences);

  type Tab = "journeys" | "account" | "settings";
  const navigation = [
    { id: "journeys", icon: "bus" },
    { id: "account", icon: "user" },
    { id: "settings", icon: "settings" },
  ] as const;
  const themes = [
    { id: "system", label: "systemTheme", icon: "monitor" },
    { id: "light", label: "lightTheme", icon: "sun" },
    { id: "dark", label: "darkTheme", icon: "moon" },
    { id: "black", label: "blackTheme", icon: "moon" },
  ] as const;

  let language = $state<Language>(startingPreferences.language);
  let theme = $state<Theme>(startingPreferences.theme);
  let accent = $state(startingPreferences.accent);
  let accentInput = $state(startingPreferences.accent.toUpperCase());
  let browserPreferencesSaved = $state(true);
  let deviceThemeSaved = $state(untrack(() => initialThemeSaved));
  let themeSave = Promise.resolve(true);
  let systemAppearanceFailed = $state(false);
  let tab = $state<Tab>("journeys");
  let busy = $state(false);
  let notice = $state<"opened" | "genericError" | "nativeUnavailable" | null>(
    null,
  );
  let status = $state<PlatformStatus | null>(null);
  let probe = $state<ProbeReport | null>(null);
  const native = isNative();
  const preferencesSaved = $derived(
    browserPreferencesSaved && deviceThemeSaved,
  );
  const t = $derived(messages[language]);
  const invalidAccent = $derived(!isAccent(accentInput));
  const canOpen = $derived(
    native &&
      status?.android === true &&
      status.providerState === "closed" &&
      !busy,
  );

  $effect(() => {
    const preferences = { language, theme, accent };
    document.documentElement.lang = language;
    applyAppearance(preferences);
    browserPreferencesSaved = savePreferences(preferences);
  });

  function selectTab(next: Tab) {
    tab = next;
    notice = null;
    window.scrollTo({ top: 0 });
  }

  function selectAccent(color: string) {
    if (!isAccent(color)) return;
    accent = color.toLowerCase();
    accentInput = color.toUpperCase();
  }

  function editAccent(value: string) {
    accentInput = value;
    if (isAccent(value)) accent = value.toLowerCase();
  }

  async function selectTheme(next: Theme) {
    if (theme === next && deviceThemeSaved) return;
    theme = next;
    if (!native) return;
    const save = themeSave
      .then(() => saveDeviceTheme(next))
      .then(
        () => true,
        () => false,
      );
    themeSave = save;
    const saved = await save;
    if (themeSave === save) deviceThemeSaved = saved;
  }

  function resetAppearance() {
    void selectTheme("system");
    selectAccent(DEFAULT_ACCENT);
  }

  async function refresh() {
    if (!native) return;
    try {
      status = await platformStatus();
    } catch {
      notice = "genericError";
    }
  }

  onMount(() => {
    const deviceTheme = matchMedia("(prefers-color-scheme: dark)");
    const syncBars = () => {
      const resolved =
        theme === "system" ? (deviceTheme.matches ? "dark" : "light") : theme;
      const color =
        resolved === "black"
          ? "#000000"
          : resolved === "dark"
            ? "#191a1c"
            : "#ffffff";
      document
        .querySelector('meta[name="theme-color"]')
        ?.setAttribute("content", color);
      if (native)
        void setSystemAppearance(resolved).then(
          () => {
            systemAppearanceFailed = false;
          },
          () => {
            systemAppearanceFailed = true;
          },
        );
    };
    // A root attribute observer also covers explicit settings changes.
    const appearanceObserver = new MutationObserver(syncBars);
    appearanceObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
    deviceTheme.addEventListener("change", syncBars);
    syncBars();
    void refresh();
    const onReturn = () => {
      if (!document.hidden) void refresh();
    };
    document.addEventListener("visibilitychange", onReturn);
    window.addEventListener("focus", onReturn);
    return () => {
      appearanceObserver.disconnect();
      deviceTheme.removeEventListener("change", syncBars);
      document.removeEventListener("visibilitychange", onReturn);
      window.removeEventListener("focus", onReturn);
    };
  });

  async function open(destination: Destination) {
    if (!canOpen) return;
    busy = true;
    notice = null;
    try {
      await openProvider(destination, language);
      notice = "opened";
    } catch {
      notice = status?.android ? "genericError" : "nativeUnavailable";
    } finally {
      busy = false;
      await refresh();
    }
  }

  async function runProbe() {
    if (!canOpen) return;
    busy = true;
    probe = null;
    notice = null;
    try {
      probe = await probeKeystore();
    } catch {
      notice = status?.android ? "genericError" : "nativeUnavailable";
    } finally {
      busy = false;
    }
  }
</script>

<a class="skip-link" href="#main">{t.skipContent}</a>

<div class="app-shell">
  <aside class="sidebar">
    <button
      class="brand"
      onclick={() => selectTab("journeys")}
      aria-label={appName}
    >
      <span class="brand-mark"><Icon name="route" size={25} /></span>
      <span class="brand-name">{appName}</span>
    </button>

    <div class="sidebar-navigation">
      <p class="nav-heading">{t.navigationHeading}</p>
      <nav aria-label={t.mainNavigation}>
        {#each navigation as item}
          <button
            class="nav-item"
            class:active={tab === item.id}
            aria-current={tab === item.id ? "page" : undefined}
            onclick={() => selectTab(item.id)}
          >
            <Icon name={item.icon} />
            <span>{t[item.id]}</span>
            {#if tab === item.id}<span class="active-dot"></span>{/if}
          </button>
        {/each}
      </nav>
    </div>

    <div class="sidebar-bottom">
      <button
        class="nav-item help-link"
        disabled={!canOpen}
        onclick={() => open("support")}
      >
        <Icon name="help" /><span>{t.help}</span><Icon
          name="external"
          size={15}
        />
      </button>
      <div class="sidebar-meta">
        <span class="environment"
          ><span class="status-dot"></span>{native
            ? t.androidLabel
            : t.browserLabel}</span
        >
        <span class="version">v0.1.0</span>
      </div>
    </div>
  </aside>

  <div class="workspace" class:journeys-workspace={tab === "journeys"}>
    <header class="workspace-header">
      <button
        class="mobile-brand"
        onclick={() => selectTab("journeys")}
        aria-label={appName}
      >
        <span class="brand-mark"><Icon name="route" size={22} /></span>
        <span>{appName}</span>
      </button>
      <div class="breadcrumb">
        <span>{appName}</span><Icon name="chevron" size={13} /><span
          >{t[tab]}</span
        >
      </div>
      {#if tab !== "journeys"}
        <div class="header-actions">
          <span class="build-badge"
            ><span class="status-dot"></span>{t.developmentLabel}</span
          >
        </div>
      {/if}
    </header>

    <main id="main" tabindex="-1">
      <div class="content">
        {#if status?.providerState === "cleanup_failed"}
          <div class="message error" role="alert">
            <Icon name="info" />
            <p>{t.cleanupFailed}</p>
          </div>
        {/if}
        {#if notice}
          <div class="message" role="status">
            <Icon name="info" />
            <p>{t[notice]}</p>
          </div>
        {/if}

        {#if tab === "journeys"}
          <div class="journeys-hero">
            <div class="journeys-intro">
              <h1>{t.onlineTimetable}</h1>
              <p>{t.onlineHint}</p>
            </div>
            <div class="transit-art" aria-hidden="true">
              <span class="transit-track track-main"></span>
              <span class="transit-track track-branch"></span>
              <span class="transit-track track-cross"></span>
              <span class="transit-station station-first"></span>
              <span class="transit-station station-transfer"></span>
              <span class="transit-station station-last"></span>
            </div>
          </div>

          <div class="journeys-grid">
            <Timetables {language} />

            <div class="journeys-links">
              <button
                class="quick-card panel"
                onclick={() => selectTab("account")}
              >
                <span class="card-icon accent-icon">
                  <Icon name="user" size={22} />
                </span>
                <strong>{t.accountTitle}</strong>
                <Icon name="arrow" size={19} />
              </button>

              <section
                class="privacy-summary panel"
                aria-labelledby="privacy-title"
              >
                <div class="privacy-heading">
                  <Icon name="shield" size={20} />
                  <h2 id="privacy-title">{t.privacy}</h2>
                </div>
                <p>{t.noJourneyHistory}</p>
                <details>
                  <summary>
                    {t.privacyDetails}<Icon name="chevron" size={14} />
                  </summary>
                  <p>{t.privacyDescription}</p>
                  <p>{t.rulesPending}</p>
                  <p>{t.limits}</p>
                </details>
              </section>
            </div>
          </div>
        {:else if tab === "account"}
          <div class="page-intro">
            <h1>{t.accountTitle}</h1>
            <p class="page-description">{t.accountDescription}</p>
          </div>
          <Account
            {language}
            canOpenProvider={canOpen}
            openLogin={() => void open("login")}
          />
        {:else}
          <div class="page-intro">
            <h1>{t.settings}</h1>
          </div>

          <section
            class="settings-panel panel"
            aria-labelledby="appearance-title"
          >
            <div class="section-heading">
              <span class="card-icon accent-icon"
                ><Icon name="palette" size={22} /></span
              >
              <div>
                <h2 id="appearance-title">{t.appearance}</h2>
              </div>
            </div>

            {#if systemAppearanceFailed}<p class="field-error" role="status">
                {t.systemAppearanceError}
              </p>{/if}
            <fieldset class="theme-fieldset">
              <legend>{t.theme}</legend>
              <div class="theme-options">
                {#each themes as option}
                  <label
                    class="theme-option"
                    class:chosen={theme === option.id}
                  >
                    <input
                      type="radio"
                      name="theme"
                      value={option.id}
                      checked={theme === option.id}
                      onchange={() => void selectTheme(option.id)}
                    />
                    <span
                      class="theme-thumbnail"
                      class:thumbnail-light={option.id === "light"}
                      class:thumbnail-dark={option.id === "dark"}
                      class:thumbnail-system={option.id === "system"}
                      class:thumbnail-black={option.id === "black"}
                      aria-hidden="true"
                    >
                      <span class="thumbnail-sidebar"
                        ><i></i><i></i><i></i></span
                      >
                      <span class="thumbnail-content"
                        ><i></i><i></i><i></i><span></span></span
                      >
                    </span>
                    <span class="theme-option-label"
                      ><Icon name={option.icon} size={17} /><span
                        >{t[option.label]}</span
                      ><span class="radio-indicator"
                        >{#if theme === option.id}<span></span>{/if}</span
                      ></span
                    >
                  </label>
                {/each}
              </div>
            </fieldset>

            <fieldset class="accent-fieldset">
              <legend>{t.accentColor}</legend>
              <div class="accent-controls">
                <div class="accent-presets">
                  {#each ACCENT_PRESETS as preset}
                    <button
                      class="accent-swatch"
                      class:selected={accent === preset.color}
                      style:--swatch={preset.color}
                      aria-label={t[preset.name]}
                      aria-pressed={accent === preset.color}
                      title={t[preset.name]}
                      onclick={() => selectAccent(preset.color)}
                      ><span
                        >{#if accent === preset.color}<Icon
                            name="check"
                            size={20}
                          />{/if}</span
                      ></button
                    >
                  {/each}
                </div>
                <div class="custom-accent">
                  <label
                    class="color-picker"
                    for="custom-accent"
                    title={t.customColor}
                  >
                    <input
                      id="custom-accent"
                      type="color"
                      value={accent}
                      oninput={(event) =>
                        selectAccent(event.currentTarget.value)}
                      aria-label={t.customColor}
                    />
                  </label>
                  <label class="hex-input-label" for="accent-hex">
                    <span>{t.customColor}</span>
                    <input
                      id="accent-hex"
                      type="text"
                      value={accentInput}
                      oninput={(event) => editAccent(event.currentTarget.value)}
                      aria-label={t.hexColor}
                      aria-invalid={invalidAccent}
                      aria-describedby={invalidAccent
                        ? "accent-error"
                        : undefined}
                      maxlength="7"
                      autocomplete="off"
                      autocapitalize="characters"
                      spellcheck="false"
                    />
                  </label>
                </div>
              </div>
              {#if invalidAccent}<p
                  id="accent-error"
                  class="field-error"
                  role="status"
                >
                  {t.invalidColor}
                </p>{/if}
            </fieldset>

            <div class="settings-actions">
              <p
                class="save-hint"
                class:save-error={!preferencesSaved}
                role="status"
              >
                <Icon
                  name={preferencesSaved ? "check" : "info"}
                  size={16}
                />{preferencesSaved ? t.preferencesSaved : t.preferencesUnsaved}
              </p>
              <button class="text-button reset-button" onclick={resetAppearance}
                >{t.resetAppearance}</button
              >
            </div>
          </section>

          <section
            class="settings-panel language-panel panel"
            aria-labelledby="language-title"
          >
            <div class="section-heading">
              <span class="card-icon"><Icon name="globe" size={22} /></span>
              <div>
                <h2 id="language-title">{t.language}</h2>
              </div>
            </div>
            <label class="sr-only" for="language">{t.language}</label>
            <select id="language" bind:value={language}
              ><option value="sk">Slovenčina</option><option value="en"
                >English</option
              ><option value="uk">Українська</option></select
            >
          </section>

          <section
            class="settings-panel panel"
            aria-labelledby="settings-privacy-title"
          >
            <div class="section-heading">
              <span class="card-icon"><Icon name="shield" size={22} /></span>
              <div>
                <h2 id="settings-privacy-title">{t.privacy}</h2>
                <p>{t.privacyDescription}</p>
              </div>
            </div>
            <div class="privacy-description">
              <p>{t.rulesPending}</p>
              <p>{t.limits}</p>
              <p>{t.session}</p>
            </div>
          </section>

          <section class="settings-panel panel" aria-labelledby="status-title">
            <div class="section-heading">
              <span class="card-icon"><Icon name="info" size={22} /></span>
              <div>
                <h2 id="status-title">{t.status}</h2>
                <p>{t.spike}</p>
              </div>
            </div>
            {#if status}
              <dl>
                <dt>Android</dt>
                <dd>{status.android ? t.yes : t.no}</dd>
                <dt>{t.verifiedRules}</dt>
                <dd>{status.optionalRuleCount}</dd>
                <dt>{t.state}</dt>
                <dd>
                  {status.providerState === "cleanup_failed"
                    ? t.cleanupFailed
                    : t[status.providerState]}
                </dd>
              </dl>
            {:else}<p class="muted">{native ? t.busy : t.localStatus}</p>{/if}
            <button
              class="secondary"
              disabled={!native || busy}
              onclick={refresh}
              ><Icon name="refresh" size={16} />{t.refresh}</button
            >
            <details class="diagnostic-details">
              <summary>{t.diagnostic}<Icon name="chevron" size={16} /></summary>
              <p>{t.diagnosticDescription}</p>
              <button class="secondary" disabled={!canOpen} onclick={runProbe}
                >{busy ? t.busy : t.runProbe}</button
              >
              {#if probe}
                <p role="status">
                  {probe.roundTrip &&
                  probe.tamperRejected &&
                  probe.uniqueNonces &&
                  probe.missingKeyRejected
                    ? t.probePassed
                    : t.probeFailed}
                </p>
                <p>{t.hardware}: {probe.hardwareBacked ? t.yes : t.no}</p>
              {/if}
            </details>
          </section>
          <button
            class="secondary"
            disabled={!canOpen}
            onclick={() => open("support")}
            >{t.help}<Icon name="external" size={16} /></button
          >
        {/if}

        <footer class="content-footer">
          <span class="footer-brand"
            ><Icon name="route" size={16} />{appName}</span
          >
          <p>{t.unofficial}</p>
          <span class="footer-version">v0.1.0</span>
        </footer>
      </div>
    </main>
  </div>

  <nav class="mobile-navigation" aria-label={t.mainNavigation}>
    {#each navigation as item}
      <button
        class:active={tab === item.id}
        aria-current={tab === item.id ? "page" : undefined}
        onclick={() => selectTab(item.id)}
        ><Icon name={item.icon} size={21} /><span>{t[item.id]}</span></button
      >
    {/each}
  </nav>
</div>
