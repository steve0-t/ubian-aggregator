<script lang="ts">
  import { onDestroy } from "svelte";
  import Icon from "./Icon.svelte";
  import {
    cancelRequest,
    isNative,
    lookupStops,
    type Stop,
  } from "./api/platform";
  import { messages, type Language } from "./i18n";
  import {
    SearchRequest,
    searchError,
    validStopQuery,
    type SearchError,
  } from "./timetable-state";

  let {
    id,
    label,
    language,
    destination = false,
    value = $bindable(""),
    selected = $bindable<Stop | null>(null),
  }: {
    id: string;
    label: string;
    language: Language;
    destination?: boolean;
    value?: string;
    selected?: Stop | null;
  } = $props();
  const t = $derived(messages[language]);
  let suggestions = $state<Stop[] | null>(null);
  let loading = $state(false);
  let error = $state<SearchError | null>(null);
  let handledValue: string | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const requests = new SearchRequest(cancelRequest);

  function reset() {
    clearTimeout(timer);
    requests.cancel();
    loading = false;
    error = null;
    suggestions = null;
  }
  function edit(next: string) {
    reset();
    handledValue = next;
    value = next;
    selected = null;
    if (validStopQuery(value) && isNative())
      timer = setTimeout(() => void lookup(), 500);
  }
  async function lookup() {
    clearTimeout(timer);
    if (!validStopQuery(value)) {
      error = "queryInvalid";
      return;
    }
    const requestedValue = value;
    const ticket = requests.begin();
    loading = true;
    error = null;
    suggestions = null;
    try {
      const stops = await lookupStops(value.trim(), requests.requestId(ticket));
      if (requests.current(ticket) && value === requestedValue)
        suggestions = stops;
    } catch (failure) {
      if (requests.current(ticket)) error = searchError(failure);
    } finally {
      if (requests.current(ticket)) {
        loading = false;
        requests.finish(ticket);
      }
    }
  }
  function choose(stop: Stop) {
    reset();
    handledValue = stop.name;
    value = stop.name;
    selected = stop;
    document.getElementById(id)?.focus();
  }
  $effect(() => {
    if (value !== handledValue || selected) {
      handledValue = value;
      reset();
    }
  });
  onDestroy(reset);
</script>

<div class="stop-field">
  <div class="stop-search">
    <span class="stop-marker" aria-hidden="true">
      {#if destination}<Icon name="pin" size={20} />{:else}<span></span>{/if}
    </span>
    <label for={id}>
      <span>{label}</span>
      <input
        {id}
        type="search"
        {value}
        placeholder={t.stopPlaceholder}
        autocomplete="off"
        maxlength="100"
        aria-describedby={`${id}-feedback`}
        oninput={(event) => edit(event.currentTarget.value)}
        onkeydown={(event) => {
          if (event.key === "Escape") reset();
        }}
      />
    </label>
    <button
      class="stop-lookup"
      type="button"
      disabled={!isNative() || loading}
      aria-label={`${t.lookupStop}: ${label}`}
      title={t.lookupStop}
      onclick={() => void lookup()}><Icon name="search" size={19} /></button
    >
  </div>
  <p id={`${id}-feedback`} class="stop-feedback muted" role="status">
    {#if loading}{t.searching}{:else if error}{t[error]}{:else if selected}<span
        class="sr-only">{t.stopSelected}</span
      >{:else if suggestions?.length === 0}{t.noStops}{:else if value}{t.selectStop}{/if}
  </p>
  {#if loading}<button
      class="secondary stop-cancel"
      type="button"
      onclick={() => {
        reset();
        error = "cancelled";
      }}>{t.cancel}</button
    >{/if}
  {#if suggestions?.length}
    <ul class="stop-suggestions" aria-label={`${t.stopSuggestions}: ${label}`}>
      {#each suggestions as stop (stop.id)}
        <li>
          <button type="button" onclick={() => choose(stop)}>{stop.name}</button
          >
        </li>
      {/each}
    </ul>
  {/if}
</div>
