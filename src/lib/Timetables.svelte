<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import Icon from "./Icon.svelte";
  import StopField from "./StopField.svelte";
  import {
    cancelRequest,
    connectionDetail,
    isNative,
    searchJourneys,
    stopDepartures,
    type ConnectionDetail,
    type Departure,
    type Journey,
    type Stop,
    type Transport,
    type TimeOccurrence,
  } from "./api/platform";
  import {
    messages,
    formatTimetableDate,
    formatTimetableTime,
    type Language,
  } from "./i18n";
  import {
    SearchRequest,
    searchError,
    timetableInput,
    type SearchError,
  } from "./timetable-state";

  let { language }: { language: Language } = $props();
  const t = $derived(messages[language]);
  const native = isNative();
  const initialTime = timetableInput();
  let originText = $state("");
  let destinationText = $state("");
  let origin = $state<Stop | null>(null);
  let destination = $state<Stop | null>(null);
  let date = $state(initialTime.date);
  let time = $state(initialTime.time);
  let arriveBy = $state(false);
  let directOnly = $state(false);
  let transport = $state<Transport>("all");
  let timeOccurrence = $state<TimeOccurrence>("earlier");
  let mode = $state<"connections" | "departureBoard">("connections");
  let loading = $state(false);
  let error = $state<SearchError | "chooseBothStops" | "chooseOrigin" | null>(
    null,
  );
  let journeys = $state<Journey[] | null>(null);
  let departures = $state<Departure[] | null>(null);
  let resultOrigin = $state("");
  let resultDestination = $state("");
  let resultMode = $state<"connections" | "departureBoard">("connections");
  let selectedJourney = $state<Journey | null>(null);
  let selectedDeparture = $state<Departure | null>(null);
  let details = $state<Record<string, ConnectionDetail>>({});
  let detailError = $state<SearchError | null>(null);
  let detailLoading = $state<string | null>(null);
  let resultsScroll = 0;
  let resultsHeading = $state<HTMLHeadingElement>();
  let returnFocus: HTMLElement | null = null;
  let detailHeading = $state<HTMLHeadingElement>();
  const requests = new SearchRequest(cancelRequest);
  const detailRequests = new SearchRequest(cancelRequest);
  const detailOpen = $derived(!!selectedJourney || !!selectedDeparture);
  const displayTime = (value: string) =>
    formatTimetableTime(new Date(value), language);
  const displayDate = (value: string) =>
    formatTimetableDate(new Date(value), language);

  function swap() {
    [origin, destination] = [destination, origin];
    [originText, destinationText] = [destinationText, originText];
  }
  function cancel() {
    requests.cancel();
    loading = false;
    error = "cancelled";
  }
  async function search(event: SubmitEvent) {
    event.preventDefault();
    requests.cancel();
    loading = false;
    if (!origin || origin.name !== originText) {
      error = mode === "connections" ? "chooseBothStops" : "chooseOrigin";
      return;
    }
    if (
      mode === "connections" &&
      (!destination || destination.name !== destinationText)
    ) {
      error = "chooseBothStops";
      return;
    }
    const ticket = requests.begin();
    const queryMode = mode;
    const queryOrigin = origin;
    const queryDestination = destination;
    const common = { date, time, transport, timeOccurrence };
    loading = true;
    error = null;
    journeys = null;
    departures = null;
    details = {};
    resultMode = queryMode;
    resultOrigin = queryOrigin.name;
    resultDestination = queryDestination?.name ?? "";
    try {
      if (queryMode === "connections" && queryDestination) {
        const response = await searchJourneys(
          {
            ...common,
            from: queryOrigin,
            to: queryDestination,
            arriveBy,
            directOnly,
          },
          requests.requestId(ticket),
        );
        if (!requests.current(ticket)) return;
        journeys = response;
        await tick();
        if (requests.current(ticket)) {
          resultsHeading?.focus({ preventScroll: true });
          resultsHeading?.scrollIntoView({
            behavior: "instant",
            block: "start",
          });
        }
      } else {
        const response = await stopDepartures(
          { ...common, stop: queryOrigin },
          requests.requestId(ticket),
        );
        if (requests.current(ticket)) departures = response;
      }
    } catch (failure) {
      if (requests.current(ticket)) error = searchError(failure);
    } finally {
      if (requests.current(ticket)) {
        loading = false;
        requests.finish(ticket);
      }
    }
  }
  async function openDetail(
    journey: Journey | null,
    departure: Departure | null,
    target: HTMLElement,
  ) {
    resultsScroll = window.scrollY;
    returnFocus = target;
    detailRequests.cancel();
    detailLoading = null;
    detailError = null;
    history.pushState({ ...history.state, timetableDetail: true }, "");
    selectedJourney = journey;
    selectedDeparture = departure;
    await tick();
    detailHeading?.focus();
    window.scrollTo({ top: 0 });
  }
  function back() {
    history.back();
  }
  async function closeDetail() {
    detailRequests.cancel();
    detailLoading = null;
    detailError = null;
    selectedJourney = null;
    selectedDeparture = null;
    await tick();
    returnFocus?.focus({ preventScroll: true });
    window.scrollTo({ top: resultsScroll });
  }
  async function loadDetail(token: string) {
    const ticket = detailRequests.begin();
    detailLoading = token;
    detailError = null;
    try {
      const response = await connectionDetail(
        token,
        detailRequests.requestId(ticket),
      );
      if (detailRequests.current(ticket))
        details = { ...details, [token]: response };
    } catch (failure) {
      if (detailRequests.current(ticket)) detailError = searchError(failure);
    } finally {
      if (detailRequests.current(ticket)) {
        detailLoading = null;
        detailRequests.finish(ticket);
      }
    }
  }
  onMount(() => {
    const onBack = () => {
      if (detailOpen) void closeDetail();
    };
    window.addEventListener("popstate", onBack);
    return () => window.removeEventListener("popstate", onBack);
  });
  onDestroy(() => {
    requests.cancel();
    detailRequests.cancel();
  });
</script>

<section class="timetable-panel panel" aria-label={t.timetables}>
  <div hidden={detailOpen}>
    <form onsubmit={search} aria-busy={loading}>
      <fieldset class="timetable-modes">
        <legend class="sr-only">{t.timetables}</legend>
        <label>
          <input
            type="radio"
            name="timetable-mode"
            value="connections"
            bind:group={mode}
          />
          <Icon name="bus" size={19} />
          <span>{t.connections}</span>
        </label>
        <label>
          <input
            type="radio"
            name="timetable-mode"
            value="departureBoard"
            bind:group={mode}
            aria-label={t.departureBoard}
          />
          <Icon name="clock" size={19} />
          <span>{t.departuresLabel}</span>
        </label>
      </fieldset>
      <div class="route-fields">
        <div class="route-stops">
          <StopField
            id="origin-stop"
            label={t.origin}
            {language}
            bind:value={originText}
            bind:selected={origin}
          />
          {#if mode === "connections"}
            <StopField
              id="destination-stop"
              label={t.destination}
              {language}
              destination
              bind:value={destinationText}
              bind:selected={destination}
            />
          {/if}
        </div>
        {#if mode === "connections"}
          <button
            type="button"
            class="swap-stops"
            aria-label={t.swapStops}
            title={t.swapStops}
            onclick={swap}
          >
            <Icon name="swap" size={20} />
          </button>
        {/if}
      </div>
      <div class="timetable-options">
        <label for="journey-date"
          >{t.date}<input
            id="journey-date"
            type="date"
            bind:value={date}
            required
          /></label
        >
        <label for="journey-time"
          >{t.time}<input
            id="journey-time"
            type="time"
            bind:value={time}
            required
          /></label
        >
        {#if mode === "connections"}
          <label for="time-direction"
            >{t.searchTime}<select id="time-direction" bind:value={arriveBy}
              ><option value={false}>{t.departure}</option><option value={true}
                >{t.arrival}</option
              ></select
            ></label
          >
        {/if}
        <label for="transport-filter"
          >{t.transport}<select id="transport-filter" bind:value={transport}
            ><option value="all">{t.allTransport}</option><option value="bus"
              >{t.bus}</option
            ><option value="train">{t.train}</option><option value="urban"
              >{t.urban}</option
            ></select
          ></label
        >
      </div>
      <div class="timetable-preferences">
        {#if mode === "connections"}
          <label class="direct-option">
            <input type="checkbox" bind:checked={directOnly} />
            <span>{t.directOnly}</span>
          </label>
        {/if}
        <details class="time-occurrence">
          <summary>{t.repeatedHour}<Icon name="chevron" size={14} /></summary>
          <p class="muted">{t.repeatedHourHint}</p>
          <label for="time-occurrence"
            >{t.timeOccurrence}<select
              id="time-occurrence"
              bind:value={timeOccurrence}
              ><option value="earlier">{t.firstOccurrence}</option><option
                value="later">{t.secondOccurrence}</option
              ></select
            ></label
          >
        </details>
      </div>
      <div class="search-actions">
        <button
          class="primary"
          type="submit"
          disabled={!native}
          aria-describedby={native ? undefined : "timetable-preview"}
        >
          <Icon name="search" size={20} />
          {mode === "connections" ? t.searchConnections : t.departureBoard}
        </button>
        {#if loading}<button class="secondary" type="button" onclick={cancel}
            >{t.cancel}</button
          >{/if}
      </div>
      <p class="muted timetable-zone">{t.timetableZone}</p>
      {#if !native}
        <p class="preview-notice" id="timetable-preview">
          <Icon name="info" size={17} />
          <span>{t.browserPreview}</span>
        </p>
      {/if}
      <p class="muted search-status" role="status">
        {#if loading}{t.searchInProgress}{:else if error}{t[error]}{/if}
      </p>
    </form>
    {#if journeys !== null || departures !== null}
      <section class="timetable-results" aria-labelledby="results-title">
        <h2 id="results-title" tabindex="-1" bind:this={resultsHeading}>
          {t.results}
        </h2>
        <p class="result-route">
          {resultOrigin}{#if resultMode === "connections"}
            → {resultDestination}{/if}
        </p>
        <p class="muted">{t.scheduled}</p>
        {#if journeys?.length === 0}<p role="status">{t.noConnections}</p>{/if}
        {#if departures?.length === 0}<p role="status">{t.noDepartures}</p>{/if}
        {#if journeys?.length}
          <ul class="journey-results">
            {#each journeys as journey (journey.id)}
              <li>
                <button
                  class="journey-result"
                  type="button"
                  onclick={(event) =>
                    void openDetail(journey, null, event.currentTarget)}
                >
                  <strong class="journey-times"
                    ><span>{displayTime(journey.departure)}</span><span
                      aria-hidden="true"
                    >
                      →
                    </span><span>{displayTime(journey.arrival)}</span></strong
                  >
                  <span
                    >{displayDate(
                      journey.departure,
                    )}{#if displayDate(journey.departure) !== displayDate(journey.arrival)}
                      → {displayDate(journey.arrival)}{/if}</span
                  >
                  <span
                    >{t.duration}: {journey.durationMinutes}
                    {t.minutes} · {t.transfers}: {journey.transfers}</span
                  >
                  {#each journey.legs as leg}<span
                      >{#if leg.service}{leg.service.line}
                        {leg.service.name} · {leg.service
                          .operator}{:else}{t.walking}{/if}</span
                    >{/each}
                  <span class="detail-link">{t.showDetails} →</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
        {#if departures?.length}
          <ul class="journey-results">
            {#each departures as item (item.id)}
              <li>
                <button
                  class="journey-result"
                  type="button"
                  onclick={(event) =>
                    void openDetail(null, item, event.currentTarget)}
                >
                  <strong class="journey-times"
                    >{displayTime(item.departure)} · {item.service.line}
                    {item.service.name}</strong
                  >
                  <span
                    >{displayDate(item.departure)} · {item.service
                      .destination}</span
                  ><span>{item.stop.name} · {item.service.operator}</span>
                  {#if item.platform}<span>{t.platform}: {item.platform}</span
                    >{/if}
                  {#if item.service.cancelled}<span>{t.serviceCancelled}</span
                    >{/if}
                  <span class="detail-link">{t.showDetails} →</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}
  </div>
  {#if detailOpen}
    <button
      class="secondary back-results"
      type="button"
      onclick={() => void back()}>← {t.backToResults}</button
    >
    <h2 tabindex="-1" bind:this={detailHeading}>{t.journeyDetail}</h2>
    <p class="muted">{t.scheduled}</p>
    {#if detailError}<p class="message" role="status">{t[detailError]}</p>{/if}
    {#if selectedJourney}
      <p>
        {displayDate(selectedJourney.departure)} · {t.duration}: {selectedJourney.durationMinutes}
        {t.minutes} · {t.transfers}: {selectedJourney.transfers}
      </p>
      <ol class="journey-legs">
        {#each selectedJourney.legs as leg, index}
          <li>
            <h3>
              {leg.service
                ? `${leg.service.line} ${leg.service.name}`
                : t.walking}
            </h3>
            {#if leg.service}<p>
                {leg.service.operator}{#if leg.service.destination}
                  · {leg.service.destination}{/if}
              </p>
              {#if leg.service.cancelled}<p>{t.serviceCancelled}</p>{/if}{/if}
            <p>
              <time datetime={leg.departure}>{displayTime(leg.departure)}</time>
              {leg.from.name}{#if leg.fromPlatform}
                · {t.platform}: {leg.fromPlatform}{/if}
            </p>
            <p>
              <time datetime={leg.arrival}>{displayTime(leg.arrival)}</time>
              {leg.to.name}{#if leg.toPlatform}
                · {t.platform}: {leg.toPlatform}{/if}
            </p>
            {#if displayDate(leg.departure) !== displayDate(leg.arrival)}<p
                class="muted"
              >
                {displayDate(leg.departure)} → {displayDate(leg.arrival)}
              </p>{/if}
            {#if !leg.service && leg.walkingMeters > 0}<p>
                {leg.walkingMeters} m
              </p>{/if}
            {#if leg.detailToken}{@render stopSequence(
                leg.detailToken,
                `leg-${index}`,
              )}{:else if leg.service}<p class="muted">
                {t.detailUnavailable}
              </p>{/if}
          </li>
        {/each}
      </ol>
    {:else if selectedDeparture}
      <h3>{selectedDeparture.service.line} {selectedDeparture.service.name}</h3>
      {#if selectedDeparture.service.cancelled}<p>{t.serviceCancelled}</p>{/if}
      <p>
        {selectedDeparture.service.operator} · {selectedDeparture.service
          .destination}
      </p>
      <p>
        {displayDate(selectedDeparture.departure)} · {displayTime(
          selectedDeparture.departure,
        )} · {selectedDeparture.stop.name}
      </p>
      {#if selectedDeparture.platform}<p>
          {t.platform}: {selectedDeparture.platform}
        </p>{/if}
      {#if selectedDeparture.detailToken}{@render stopSequence(
          selectedDeparture.detailToken,
          "departure-detail",
        )}{:else}<p class="muted">{t.detailUnavailable}</p>{/if}
    {/if}
  {/if}
</section>

{#snippet stopSequence(token: string, id: string)}
  {#if details[token]}
    <h4 {id}>{t.intermediateStops}</h4>
    <ol class="service-stops" aria-labelledby={id}>
      {#each details[token].stops as item}<li>
          <time datetime={item.departure}>{displayTime(item.departure)}</time>
          {item.stop.name}{#if item.platform}
            · {t.platform}: {item.platform}{/if}
        </li>{/each}
    </ol>
  {:else}
    <button
      class="secondary"
      type="button"
      disabled={detailLoading === token}
      onclick={() => void loadDetail(token)}
      >{detailLoading === token ? t.busy : t.loadStopSequence}</button
    >
    {#if detailLoading === token}<button
        class="secondary"
        type="button"
        onclick={() => {
          detailRequests.cancel();
          detailLoading = null;
          detailError = "cancelled";
        }}>{t.cancel}</button
      >{/if}
  {/if}
{/snippet}
