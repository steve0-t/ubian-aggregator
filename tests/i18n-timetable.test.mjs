import test from "node:test";
import assert from "node:assert/strict";
import {
  messages,
  formatTimetableTime,
  formatTimetableDate,
} from "../src/lib/i18n/index.ts";
import {
  SearchRequest,
  searchError,
  validStopQuery,
  timetableInput,
} from "../src/lib/timetable-state.ts";

test("all three dictionaries have complete, nonempty messages", () => {
  const keys = Object.keys(messages.sk).sort();
  assert.deepEqual(Object.keys(messages).sort(), ["en", "sk", "uk"]);
  for (const dictionary of Object.values(messages)) {
    assert.deepEqual(Object.keys(dictionary).sort(), keys);
    for (const text of Object.values(dictionary)) assert.ok(text.trim().length);
  }
  assert.match(messages.uk.onlineTimetable, /Ubian/);
  assert.match(messages.uk.providerLanguage, /словацька/);
});

test("timetable formatting preserves Bratislava midnight and DST across device zones", () => {
  // Explicit UTC instants avoid the host timezone or locale parsing a wall time.
  for (const language of ["sk", "en", "uk"]) {
    for (const [instant, expected] of [
      ["2026-11-16T23:15:00Z", "00:15"],
      ["2026-03-29T00:30:00Z", "01:30"],
      ["2026-03-29T01:30:00Z", "03:30"],
      ["2026-10-25T00:30:00Z", "02:30"],
      ["2026-10-25T01:30:00Z", "02:30"],
    ])
      assert.equal(formatTimetableTime(new Date(instant), language), expected);
    assert.match(
      formatTimetableDate(new Date("2026-11-16T23:15:00Z"), language),
      /17/,
    );
  }
  assert.match(
    formatTimetableDate(new Date("2026-11-16T23:15:00Z"), "uk"),
    /листопада/,
  );
});

test("cancellation and superseded searches reject late completions", () => {
  const requests = new SearchRequest();
  const first = requests.begin();
  requests.cancel();
  assert.equal(requests.current(first), false);
  const second = requests.begin();
  const third = requests.begin();
  assert.equal(requests.current(second), false);
  assert.equal(requests.current(third), true);
});

test("search errors are local message keys and never raw provider content", () => {
  for (const [code, expected] of Object.entries({
    source_unavailable: "providerUnavailable",
    invalid_query: "queryInvalid",
    offline: "offline",
    timeout: "timeout",
    rate_limited: "rateLimited",
    provider_changed: "providerChanged",
    android_required: "nativeUnavailable",
    invalid_time: "invalidTime",
    ambiguous_time: "invalidTime",
    detail_unavailable: "detailUnavailable",
    cancelled: "cancelled",
  }))
    assert.equal(searchError(code), expected);
  assert.equal(
    searchError("<script>private server data</script>"),
    "genericError",
  );
  assert.equal(searchError({ message: "provider response" }), "genericError");
  for (const query of ["", "a", "a\n", "xx\u0000", "x".repeat(101)])
    assert.equal(validStopQuery(query), false);
  for (const query of ["  Bratislava, Račianske mýto  ", "Київ", "🚆🚆"])
    assert.equal(validStopQuery(query), true);
});

test("request replacement aborts native work, finishing leaves no pending abort", async () => {
  const aborted = [];
  const requests = new SearchRequest(async (id) => {
    aborted.push(id);
  });
  const first = requests.begin();
  const firstId = requests.requestId(first);
  const second = requests.begin();
  const secondId = requests.requestId(second);
  assert.equal(requests.current(first), false);
  assert.deepEqual(aborted, [firstId]);
  assert.notEqual(firstId, secondId);
  requests.finish(second);
  requests.cancel();
  assert.deepEqual(aborted, [firstId]);
});

test("default search inputs use Bratislava civil time independently of device zone", () => {
  assert.deepEqual(timetableInput(new Date("2026-11-16T23:15:00Z")), {
    date: "2026-11-17",
    time: "00:15",
  });
  assert.deepEqual(timetableInput(new Date("2026-03-29T01:30:00Z")), {
    date: "2026-03-29",
    time: "03:30",
  });
});
