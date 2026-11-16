<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { name as appName } from "../package.json";
  import Icon from "./lib/Icon.svelte";
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
    type Destination,
    type PlatformStatus,
    type ProbeReport,
  } from "./lib/api/platform";

  let { initialPreferences }: { initialPreferences: Preferences } = $props();
  const startingPreferences = untrack(() => initialPreferences);

  type Tab = "journeys" | "account" | "settings";
  const navigation = [
    { id: "journeys", icon: "route" },
    { id: "account", icon: "user" },
    { id: "settings", icon: "settings" },
  ] as const;
  const themes = [
    { id: "system", label: "systemTheme", icon: "monitor" },
    { id: "light", label: "lightTheme", icon: "sun" },
    { id: "dark", label: "darkTheme", icon: "moon" },
  ] as const;

  let language = $state<Language>(startingPreferences.language);
  let theme = $state<Theme>(startingPreferences.theme);
  let accent = $state(startingPreferences.accent);
  let accentInput = $state(startingPreferences.accent.toUpperCase());
  let preferencesSaved = $state(true);
  let tab = $state<Tab>("journeys");
  let busy = $state(false);
  let notice = $state<"opened" | "genericError" | "nativeUnavailable" | null>(
    null,
  );
  let status = $state<PlatformStatus | null>(null);
  let probe = $state<ProbeReport | null>(null);
  const native = isNative();
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
    preferencesSaved = savePreferences(preferences);
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

  function resetAppearance() {
    theme = "system";
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
    void refresh();
    const onReturn = () => {
      if (!document.hidden) void refresh();
    };
    document.addEventListener("visibilitychange", onReturn);
    window.addEventListener("focus", onReturn);
    return () => {
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

  <div class="workspace">
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
      <div class="header-actions">
        <span class="build-badge"
          ><span class="status-dot"></span>{t.developmentLabel}</span
        >
        <button
          class="icon-button"
          aria-label={t.personalize}
          title={t.personalize}
          onclick={() => selectTab("settings")}><Icon name="palette" /></button
        >
      </div>
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
          <div class="page-intro">
            <h1>{t.journeys}</h1>
          </div>

          <section class="journey-panel panel" aria-labelledby="journey-title">
            <div class="journey-copy">
              <h2 id="journey-title">{t.timetables}</h2>
              <p>{t.journeyDescription}</p>
              <button
                class="primary"
                disabled={!canOpen}
                onclick={() => open("journeys")}
              >
                {busy ? t.busy : t.openJourneys}<Icon name="arrow" size={18} />
              </button>
              <span class="guest-hint"
                ><Icon name="user" size={15} />{t.guestShort}</span
              >
            </div>
            <div class="route-art" aria-hidden="true">
              <svg viewBox="0 0 300 270" fill="none">
                <defs>
                  <pattern
                    id="route-grid"
                    x="8"
                    y="8"
                    width="22"
                    height="22"
                    patternUnits="userSpaceOnUse"
                  >
                    <circle
                      cx="1"
                      cy="1"
                      r="1"
                      fill="currentColor"
                      opacity=".16"
                    />
                  </pattern>
                </defs>
                <rect
                  x="4"
                  y="4"
                  width="292"
                  height="262"
                  rx="24"
                  fill="url(#route-grid)"
                />
                <circle
                  cx="151"
                  cy="137"
                  r="99"
                  stroke="currentColor"
                  opacity=".08"
                />
                <circle
                  cx="151"
                  cy="137"
                  r="72"
                  stroke="currentColor"
                  opacity=".08"
                />
                <path
                  d="M80 75h99c48 0 48 61 0 61h-47c-51 0-51 61 0 61h87"
                  stroke="var(--accent-border)"
                  stroke-width="16"
                  stroke-linecap="round"
                />
                <path
                  d="M80 75h99c48 0 48 61 0 61h-47c-51 0-51 61 0 61h87"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-dasharray="4 6"
                />
                <rect
                  x="48"
                  y="43"
                  width="64"
                  height="64"
                  rx="19"
                  fill="var(--surface)"
                  stroke="var(--accent-border)"
                />
                <circle
                  cx="80"
                  cy="75"
                  r="14"
                  fill="var(--accent-soft)"
                  stroke="currentColor"
                  stroke-width="1.7"
                />
                <circle cx="80" cy="75" r="5" fill="currentColor" />
                <rect
                  x="187"
                  y="165"
                  width="64"
                  height="64"
                  rx="19"
                  fill="var(--accent)"
                />
                <path
                  d="M230 193c0 8-11 17-11 17s-11-9-11-17a11 11 0 1 1 22 0Z"
                  stroke="var(--on-accent)"
                  stroke-width="1.7"
                />
                <circle
                  cx="219"
                  cy="193"
                  r="3.5"
                  stroke="var(--on-accent)"
                  stroke-width="1.7"
                />
                <circle
                  cx="152"
                  cy="136"
                  r="5"
                  fill="var(--surface)"
                  stroke="currentColor"
                  stroke-width="2"
                />
                <path
                  d="m253 64 3 7 7 3-7 3-3 7-3-7-7-3 7-3 3-7ZM51 177v10m-5-5h10"
                  stroke="currentColor"
                  stroke-width="1.5"
                  opacity=".55"
                />
              </svg>
            </div>
          </section>

          {#if !native}
            <div class="preview-notice">
              <Icon name="monitor" size={18} />
              <p>{t.browserPreview}</p>
            </div>
          {/if}

          <div class="quick-links">
            <button
              class="quick-card panel"
              onclick={() => selectTab("account")}
            >
              <span class="card-icon"><Icon name="user" size={22} /></span>
              <span class="quick-card-copy"
                ><strong>{t.accountTitle}</strong></span
              >
              <Icon name="chevron" size={18} />
            </button>
            <button
              class="quick-card panel"
              onclick={() => selectTab("settings")}
            >
              <span class="card-icon accent-icon"
                ><Icon name="palette" size={22} /></span
              >
              <span class="quick-card-copy"
                ><strong>{t.appearance}</strong></span
              >
              <Icon name="chevron" size={18} />
            </button>
          </div>

          <section class="privacy-summary" aria-labelledby="privacy-title">
            <span class="privacy-icon"><Icon name="shield" size={23} /></span>
            <div>
              <h2 id="privacy-title">{t.privacy}</h2>
              <p>{t.privacyDescription}</p>
              <details>
                <summary
                  >{t.privacyDetails}<Icon name="chevron" size={14} /></summary
                >
                <p>{t.rulesPending}</p>
                <p>{t.limits}</p>
              </details>
            </div>
          </section>
          <p class="return-hint">
            <Icon name="info" size={15} />{t.returnHint}
          </p>
        {:else if tab === "account"}
          <div class="page-intro">
            <h1>{t.accountTitle}</h1>
            <p class="page-description">{t.accountDescription}</p>
          </div>
          <section class="account-panel panel" aria-labelledby="account-title">
            <span class="account-avatar"><Icon name="user" size={30} /></span>
            <span class="provider-badge"
              ><span class="status-dot"></span>www.ubian.sk/login</span
            >
            <h2 id="account-title">{t.signIn}</h2>
            <button
              class="primary"
              disabled={!canOpen}
              onclick={() => open("login")}
            >
              {busy ? t.busy : t.openLogin}<Icon name="external" size={17} />
            </button>
          </section>
          {#if !native}
            <div class="preview-notice">
              <Icon name="monitor" size={18} />
              <p>{t.browserPreview}</p>
            </div>
          {/if}
          <div class="information-panel panel">
            {#each [{ icon: "key", title: t.passwordTitle, description: t.credentials }, { icon: "clock", title: t.sessionTitle, description: t.session }, { icon: "external", title: t.socialTitle, description: t.social }] as item}
              <div class="information-row">
                <span class="small-icon"
                  ><Icon
                    name={item.icon as "key" | "clock" | "external"}
                  /></span
                >
                <div>
                  <h2>{item.title}</h2>
                  <p>{item.description}</p>
                </div>
              </div>
            {/each}
          </div>
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
                      bind:group={theme}
                    />
                    <span
                      class="theme-thumbnail"
                      class:thumbnail-light={option.id === "light"}
                      class:thumbnail-dark={option.id === "dark"}
                      class:thumbnail-system={option.id === "system"}
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
              ></select
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
