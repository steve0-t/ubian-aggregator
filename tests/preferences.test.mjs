import test from "node:test";
import assert from "node:assert/strict";
import {
  ACCENT_PRESETS,
  DEFAULT_ACCENT,
  DEFAULT_PREFERENCES,
  PREFERENCES_KEY,
  accentTokens,
  readPreferences,
  savePreferences,
} from "../src/lib/preferences.ts";

function memoryStorage(initial = null) {
  let value = initial;
  return {
    getItem(key) {
      assert.equal(key, PREFERENCES_KEY);
      return value;
    },
    setItem(key, next) {
      assert.equal(key, PREFERENCES_KEY);
      value = next;
    },
  };
}

test("appearance and language preferences survive saving and reloading", () => {
  const storage = memoryStorage();
  const preferences = { accent: "#ab12ef", theme: "dark", language: "en" };
  assert.equal(savePreferences(preferences, storage), true);
  assert.deepEqual(readPreferences(storage), preferences);
  assert.deepEqual(
    Object.keys(JSON.parse(storage.getItem(PREFERENCES_KEY))).sort(),
    ["accent", "language", "theme"],
  );
});

test("missing, corrupt, and unexpected stored data falls back to usable defaults", () => {
  for (const value of [null, "{", "null", "false", "[]", '"dark"', "42"]) {
    assert.deepEqual(
      readPreferences(memoryStorage(value)),
      DEFAULT_PREFERENCES,
    );
  }
  assert.deepEqual(
    readPreferences(
      memoryStorage(
        JSON.stringify({
          accent: "url(https://example.com)",
          theme: "unknown",
          language: "xx",
        }),
      ),
    ),
    DEFAULT_PREFERENCES,
  );
  assert.deepEqual(
    readPreferences(
      memoryStorage(
        JSON.stringify({
          accent: "#AABBCC",
          theme: "light",
          language: "en",
        }),
      ),
    ),
    { accent: "#aabbcc", theme: "light", language: "en" },
  );
  assert.deepEqual(
    readPreferences(
      memoryStorage(
        JSON.stringify({
          accent: "#bad",
          theme: "dark",
          language: "en",
        }),
      ),
    ),
    { accent: DEFAULT_ACCENT, theme: "dark", language: "en" },
  );
});

test("unavailable or full storage does not prevent using the interface", () => {
  const unavailable = {
    getItem() {
      throw new Error("storage blocked");
    },
    setItem() {
      throw new Error("storage full");
    },
  };
  assert.deepEqual(readPreferences(unavailable), DEFAULT_PREFERENCES);
  assert.equal(savePreferences({ ...DEFAULT_PREFERENCES }, unavailable), false);
});

// Calculate WCAG contrast independently of the color-generation implementation.
function luminance(hex) {
  const rgb = hex
    .match(/[0-9a-f]{2}/gi)
    .map((part) => parseInt(part, 16) / 255);
  const [r, g, b] = rgb.map((v) =>
    v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4,
  );
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function assertReadable(text, background) {
  const values = [luminance(text), luminance(background)].sort((a, b) => a - b);
  const ratio = (values[1] + 0.05) / (values[0] + 0.05);
  assert.ok(
    ratio >= 4.5,
    `${text} on ${background}: contrast ${ratio.toFixed(2)}`,
  );
}

test("custom accents keep buttons and highlighted text readable in every palette", () => {
  const colors = new Set(ACCENT_PRESETS.map(({ color }) => color));
  // Cover the RGB gamut, including black, white, saturated and pale colors.
  for (const r of [0, 51, 102, 153, 204, 255]) {
    for (const g of [0, 51, 102, 153, 204, 255]) {
      for (const b of [0, 51, 102, 153, 204, 255]) {
        colors.add(
          `#${[r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")}`,
        );
      }
    }
  }
  for (const color of colors) {
    const tokens = accentTokens(color);
    assert.equal(tokens["--accent"], color);
    assertReadable(tokens["--on-accent"], color);
    for (const background of [
      "#ffffff",
      "#f7f8f8",
      tokens["--accent-light-soft"],
    ]) {
      assertReadable(tokens["--accent-light-text"], background);
    }
    for (const background of [
      "#191a1c",
      "#202123",
      "#2a2b2e",
      tokens["--accent-dark-soft"],
    ]) {
      assertReadable(tokens["--accent-dark-text"], background);
    }
    for (const background of [
      "#000000",
      "#0d0d0d",
      "#141414",
      "#1b1b1b",
      tokens["--accent-black-soft"],
    ]) {
      assertReadable(tokens["--accent-black-text"], background);
    }
  }
});

test("invalid custom accents cannot become CSS values", () => {
  for (const value of ["#fff", "red", "#gg0000", "#000000; color: red", ""]) {
    assert.equal(accentTokens(value)["--accent"], DEFAULT_ACCENT);
  }
});

test("Ukrainian and true black persist without migrating existing preferences", () => {
  for (const theme of ["system", "light", "dark", "black"]) {
    for (const language of ["sk", "en", "uk"]) {
      const storage = memoryStorage();
      const preferences = { accent: "#123456", theme, language };
      assert.equal(savePreferences(preferences, storage), true);
      assert.deepEqual(readPreferences(storage), preferences);
    }
  }
});
