// Optional browser check; Playwright and Chromium are supplied by the caller.
const { chromium } = await import(
  process.env.PLAYWRIGHT_MODULE ?? "playwright"
);
import assert from "node:assert/strict";
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_EXECUTABLE,
  headless: true,
  args: ["--no-sandbox", "--disable-dev-shm-usage"],
});
try {
  const page = await browser.newPage({
    viewport: { width: 360, height: 800 },
    hasTouch: true,
    isMobile: true,
    colorScheme: "light",
  });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(process.env.UBIAN_UI_URL ?? "http://127.0.0.1:1422", {
    waitUntil: "domcontentloaded",
    timeout: 60000,
  });
  await page.locator(".mobile-navigation button").nth(2).click();
  await page.locator("#language").selectOption("uk");
  await page
    .locator(".theme-option")
    .filter({ has: page.locator('input[value="black"]') })
    .click();
  await page.waitForFunction(
    () =>
      document.documentElement.dataset.theme === "black" &&
      document.documentElement.lang === "uk",
  );
  const backgrounds = await page.evaluate(() =>
    ["body", ".mobile-navigation", ".workspace-header"].map(
      (s) => getComputedStyle(document.querySelector(s)).backgroundColor,
    ),
  );
  assert.equal(backgrounds[0], "rgb(0, 0, 0)");
  assert.equal(backgrounds[1], "rgb(0, 0, 0)");
  await page.emulateMedia({ colorScheme: "dark" });
  assert.equal(await page.locator("html").getAttribute("data-theme"), "black");
  await page.reload();
  await page.locator(".mobile-navigation button").nth(2).click();
  assert.equal(await page.locator("#language").inputValue(), "uk");
  assert.equal(
    await page.locator('input[name="theme"]:checked').inputValue(),
    "black",
  );
  for (const theme of ["system", "light", "dark", "black"]) {
    await page
      .locator(".theme-option")
      .filter({ has: page.locator(`input[value="${theme}"]`) })
      .click();
    await page.locator("#accent-hex").fill("#FFFFFF");
    await page.locator("#accent-hex").fill("#000000");
    await page.locator("#accent-hex").fill("#CF386F");
    const nav = page.locator(".mobile-navigation button").first();
    assert.equal(
      await nav.evaluate((el) => getComputedStyle(el).webkitTapHighlightColor),
      "rgba(0, 0, 0, 0)",
    );
    assert.equal(
      await nav.evaluate((el) => getComputedStyle(el).userSelect),
      "none",
    );
  }
  await page.evaluate(() => (document.documentElement.style.fontSize = "200%"));
  const overflow = await page.evaluate(
    () =>
      document.documentElement.scrollWidth >
      document.documentElement.clientWidth,
  );
  assert.equal(overflow, false, "Ukrainian settings overflow at 360px / 200%");

  await page.evaluate(() => window.scrollTo(0, 0));
  await page.locator(".mobile-navigation button").first().click();
  assert.equal(
    await page.locator("main").getByText("Вигляд", { exact: true }).count(),
    0,
  );
  assert.equal(await page.locator(".header-actions button").count(), 0);
  await page.locator(".timetable-panel").waitFor();
  assert.equal(await page.getByText("Онлайн-розклади Ubian").count(), 0);

  await page.setViewportSize({ width: 1280, height: 900 });
  await page.evaluate(() => (document.documentElement.style.fontSize = "100%"));
  await page.locator(".sidebar nav button").nth(2).click();
  const focusTarget = page.locator(".sidebar nav button").nth(1);
  await focusTarget.focus();
  // Keyboard interaction keeps focus and aria-current.
  await page.keyboard.press("Enter");
  assert.equal(await focusTarget.getAttribute("aria-current"), "page");
  assert.equal(
    await focusTarget.evaluate((el) => el.matches(":focus-visible")),
    true,
  );
  assert.equal(
    await focusTarget.evaluate((el) => getComputedStyle(el).outlineStyle),
    "solid",
  );
  const box = await focusTarget.boundingBox();
  await page.mouse.move(box.x + 15, box.y + 15);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width + 40, box.y + box.height + 40);
  await page.mouse.up();
  assert.equal(
    await focusTarget.evaluate((el) => el.matches(":active")),
    false,
  );
  assert.equal(
    await page.evaluate(
      () =>
        document.documentElement.scrollWidth >
        document.documentElement.clientWidth,
    ),
    false,
  );
  await page.setViewportSize({ width: 320, height: 800 });
  await page.evaluate(() => (document.documentElement.style.fontSize = "200%"));
  await page.locator(".mobile-navigation button").nth(2).click();
  assert.equal(
    await page.evaluate(
      () =>
        document.documentElement.scrollWidth >
        document.documentElement.clientWidth,
    ),
    false,
    "320px / 200% text overflow",
  );
  assert.deepEqual(errors, []);
  console.log(
    "Chromium: Ukrainian selection/reload, all themes, black base/navigation, custom accents, 320/360px at 200% text, scoped tap/select CSS, keyboard focus and release-off-control checks passed. Android interactions remain unverified.",
  );
} finally {
  await browser.close();
}
