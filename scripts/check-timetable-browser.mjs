// UI regression harness. These synthetic IPC responses are test-only; production
// always uses the Rust adapter and has no fixture fallback.
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
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.addInitScript(() => {
    const origin = { id: 1321, name: "Dvorianky, č.d.143" };
    const destination = { id: 8195, name: "Košice, AS" };
    const service = {
      line: "TEST-073104",
      name: "Test service",
      operator: "Test operator",
      destination: destination.name,
      cancelled: false,
    };
    const journey = {
      id: "test-journey",
      departure: "2026-11-18T06:44:00+01:00",
      arrival: "2026-11-18T07:40:00+01:00",
      durationMinutes: 56,
      transfers: 0,
      legs: [
        {
          from: origin,
          to: destination,
          departure: "2026-11-18T06:44:00+01:00",
          arrival: "2026-11-18T07:40:00+01:00",
          fromPlatform: "2",
          toPlatform: null,
          walkingMeters: 0,
          service,
          detailToken: "test-detail-token",
        },
      ],
    };
    const state = (window.__timetableTest = {
      calls: [],
      searchMode: "success",
      lookupMode: "success",
      pending: null,
      lookupPending: null,
      journey,
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
            case "cancel_request":
              return;
            case "lookup_stops":
              if (state.lookupMode === "pending")
                return new Promise((resolve) => {
                  state.lookupPending = resolve;
                });
              return args.request.query.toLowerCase().includes("dvorianky")
                ? [origin]
                : [destination];
            case "search_journeys":
              if (state.searchMode === "pending")
                return new Promise((resolve) => {
                  state.pending = resolve;
                });
              if (state.searchMode === "offline") throw "offline";
              if (state.searchMode === "empty") return [];
              return [journey];
            case "departures":
              return [
                {
                  id: "test-departure",
                  stop: origin,
                  departure: journey.departure,
                  platform: "2",
                  service,
                  detailToken: "test-detail-token",
                },
              ];
            case "connection_detail":
              throw "detail_unavailable";
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
  const search = page.getByRole("button", {
    name: "Vyhľadať spojenie",
    exact: true,
  });
  const origin = page.locator("#origin-stop");
  const destination = page.locator("#destination-stop");
  await origin.fill("Dvorianky");
  await page
    .getByRole("button", { name: "Dvorianky, č.d.143", exact: true })
    .click();
  await destination.fill("Kosice");
  await page.getByRole("button", { name: "Košice, AS", exact: true }).click();
  await page.locator("#journey-date").fill("2026-11-18");
  await page.locator("#journey-time").fill("06:00");
  await page.locator("#transport-filter").selectOption("bus");
  await page.getByLabel("Bez prestupov", { exact: true }).check();
  await search.click();
  const result = page.locator(".journey-result");
  await result.waitFor();
  const submitted = await page.evaluate(
    () =>
      window.__timetableTest.calls.find(
        (call) => call.name === "search_journeys",
      ).args,
  );
  assert.equal(submitted.request.from.id, 1321);
  assert.equal(submitted.request.to.id, 8195);
  assert.equal(submitted.request.directOnly, true);
  assert.equal(submitted.request.transport, "bus");
  assert.equal(submitted.request.date, "2026-11-18");
  assert.equal(submitted.request.time, "06:00");
  assert.ok(submitted.requestId);
  await result.scrollIntoViewIfNeeded();
  const scroll = await page.evaluate(() => window.scrollY);
  await result.click();
  await page.locator(".journey-legs").waitFor();
  await page.getByRole("button", { name: "Zobraziť zastávky spoja" }).click();
  await page
    .getByText("Doplnenie detailu nie je dostupné.", { exact: false })
    .waitFor();
  assert.equal(await page.locator(".journey-legs").count(), 1);
  await page
    .getByRole("button", { name: "Späť na výsledky", exact: false })
    .click();
  await result.waitFor();
  await page.waitForFunction(
    (expected) => Math.abs(window.scrollY - expected) < 2,
    scroll,
  );
  assert.equal(await origin.inputValue(), "Dvorianky, č.d.143");
  assert.equal(await page.locator("#transport-filter").inputValue(), "bus");
  await result.click();
  await page.locator(".journey-legs").waitFor();
  await page.goBack();
  await result.waitFor();
  // Editing a resolved name invalidates its provider ID, even if edited back.
  await origin.fill("Dvorianky, č.d.143 ");
  await search.click();
  await page
    .getByText("Vyberte obe zastávky zo zoznamu.", { exact: true })
    .waitFor();
  const before = await page.evaluate(
    () =>
      window.__timetableTest.calls.filter(
        (call) => call.name === "search_journeys",
      ).length,
  );
  assert.equal(before, 1);
  await page
    .getByRole("button", { name: "Dvorianky, č.d.143", exact: true })
    .click();
  // Cancel at the Rust boundary; a late resolved IPC promise cannot show results.
  await page.evaluate(() => {
    window.__timetableTest.searchMode = "pending";
  });
  await search.click();
  await page.waitForFunction(() => window.__timetableTest.pending !== null);
  await page.getByRole("button", { name: "Zrušiť", exact: true }).click();
  await page.evaluate(() => {
    window.__timetableTest.pending([window.__timetableTest.journey]);
  });
  await page.getByText("Vyhľadávanie bolo zrušené.", { exact: true }).waitFor();
  assert.equal(await result.count(), 0);
  assert.ok(
    await page.evaluate(() =>
      window.__timetableTest.calls.some(
        (call) => call.name === "cancel_request",
      ),
    ),
  );
  // Even an invalid replacement submit cancels the older valid request.
  await search.click();
  await origin.fill("Dvorianky unresolved");
  await search.click();
  await page.evaluate(() => {
    window.__timetableTest.pending([window.__timetableTest.journey]);
  });
  await page
    .getByText("Vyberte obe zastávky zo zoznamu.", { exact: true })
    .waitFor();
  assert.equal(await result.count(), 0);
  await page
    .getByRole("button", { name: "Dvorianky, č.d.143", exact: true })
    .click();
  // A new request fences a previous request even when it finishes afterwards.
  await search.click();
  await page.evaluate(() => {
    window.__timetableTest.searchMode = "success";
  });
  await search.click();
  await result.waitFor();
  await page.evaluate(() => {
    const state = window.__timetableTest;
    state.pending([{ ...state.journey, id: "stale", legs: [] }]);
  });
  await page.waitForTimeout(50);
  assert.match(await result.innerText(), /TEST-073104/);
  await page.evaluate(() => {
    window.__timetableTest.searchMode = "offline";
  });
  await search.click();
  await page
    .getByText("Nie ste pripojení. Skontrolujte pripojenie a skúste to znova.")
    .waitFor();
  await page.evaluate(() => {
    window.__timetableTest.searchMode = "empty";
  });
  await search.click();
  await page
    .getByText("Pre toto vyhľadávanie neboli vrátené žiadne spojenia.")
    .waitFor();
  await page.getByLabel("Odchody zo zastávky", { exact: true }).check();
  await page
    .getByRole("button", { name: "Odchody zo zastávky", exact: true })
    .click();
  await result.waitFor();
  assert.match(await result.innerText(), /06:44/);
  assert.ok(
    await page.evaluate(() =>
      window.__timetableTest.calls.some(
        (call) =>
          call.name === "departures" && call.args.request.stop.id === 1321,
      ),
    ),
  );
  // External swap invalidates an old autocomplete and fences its completion.
  await page.getByLabel("Spojenia", { exact: true }).check();
  await page.evaluate(() => {
    window.__timetableTest.lookupMode = "pending";
  });
  await destination.fill("unfinished");
  await page.waitForFunction(
    () => window.__timetableTest.lookupPending !== null,
  );
  await page
    .getByRole("button", { name: "Vymeniť zastávky", exact: false })
    .click();
  await page.evaluate(() =>
    window.__timetableTest.lookupPending([
      { id: 999, name: "Stale suggestion" },
    ]),
  );
  await page.waitForTimeout(50);
  assert.equal(
    await page.getByText("Stale suggestion", { exact: true }).count(),
    0,
  );
  assert.equal(await origin.inputValue(), "unfinished");
  assert.equal(await destination.inputValue(), "Dvorianky, č.d.143");
  // Local storage contains appearance preferences only, never query/history data.
  const stored = await page.evaluate(() =>
    Object.values(localStorage).join(" "),
  );
  assert.doesNotMatch(stored, /Dvorianky|Košice|1321|8195/);
  await page.setViewportSize({ width: 320, height: 800 });
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "200%";
  });
  assert.equal(
    await page.evaluate(
      () =>
        document.documentElement.scrollWidth >
        document.documentElement.clientWidth,
    ),
    false,
    "Native timetable layout overflow at 320px / 200% text",
  );
  assert.deepEqual(errors, []);
  console.log(
    "Timetable UI: stop selection/edit/swap, native query DTOs, detail/date limitation, button/browser Back with scroll, cancellation/late completions, supersession, offline/empty results, departure board and no persisted journey history passed using test-only IPC responses.",
  );
} finally {
  await browser.close();
}
