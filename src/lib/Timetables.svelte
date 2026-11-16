<script lang="ts">
  import { onDestroy } from "svelte";
  import { lookupStops, isNative, type Stop } from "./api/platform";
  import { messages, type Language } from "./i18n";
  import { SearchRequest, searchError, validStopQuery, type SearchError } from "./timetable-state";

  let { language }: { language: Language } = $props();
  const t = $derived(messages[language]);
  let query = $state("");
  let loading = $state(false);
  let error = $state<SearchError | null>(null);
  let stops = $state<Stop[] | null>(null);
  const requests = new SearchRequest();

  function cancel() {
    requests.cancel();
    loading = false;
    error = "cancelled";
  }
  async function search(event: SubmitEvent) {
    event.preventDefault();
    if (!validStopQuery(query)) { error = "queryInvalid"; return; }
    const ticket = requests.begin();
    loading = true;
    error = null;
    stops = null;
    try {
      const response = await lookupStops(query.trim());
      if (requests.current(ticket)) stops = response;
    } catch (failure) {
      if (requests.current(ticket)) error = searchError(failure);
    } finally {
      if (requests.current(ticket)) loading = false;
    }
  }
  onDestroy(() => requests.cancel());
</script>

<section class="timetable-panel panel" aria-labelledby="timetable-title">
  <h2 id="timetable-title">{t.timetables}</h2>
  <div class="message" role="status">
    <div><h3>{t.timetableIncomplete}</h3><p>{t.timetableBlocked}</p></div>
  </div>
  <form onsubmit={search} aria-busy={loading}>
    <label for="stop-query">{t.stopQuery}</label>
    <div class="stop-search">
      <input id="stop-query" type="search" bind:value={query} autocomplete="off"
        aria-invalid={error === "queryInvalid"} aria-describedby="stop-feedback" />
      <button class="primary" disabled={loading || !isNative()} type="submit">{t.lookupStop}</button>
      {#if loading}<button class="secondary" type="button" onclick={cancel}>{t.cancel}</button>{/if}
    </div>
    <p id="stop-feedback" role="status" class="muted">
      {#if loading}{t.searching}{:else if error}{t[error]}{:else if stops?.length === 0}{t.noStops}{/if}
    </p>
  </form>
  {#if stops?.length}
    <ul>{#each stops as stop (stop.id)}<li>{stop.name}</li>{/each}</ul>
  {/if}
  <p class="muted">{t.noJourneyHistory}</p>
</section>
