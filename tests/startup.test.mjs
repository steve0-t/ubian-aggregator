import test from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import { readFileSync } from "node:fs";
const script = readFileSync(new URL("../public/appearance-init.js", import.meta.url), "utf8");

test("blocking startup paints saved black before app code in either device theme", () => {
  for (const dark of [false, true]) {
    const root = { dataset: {}, style: {}, lang: "" };
    vm.runInNewContext(script, {
      document: { documentElement: root },
      localStorage: { getItem: () => JSON.stringify({ theme: "black", language: "uk" }) },
      matchMedia: () => ({ matches: dark }),
    });
    assert.equal(root.style.backgroundColor, "#000000");
    assert.equal(root.dataset.theme, "black");
    assert.equal(root.style.colorScheme, "dark");
    assert.equal(root.lang, "uk");
  }
});

test("startup preserves system default and tolerates inaccessible preferences", () => {
  for (const dark of [false, true]) {
    const root = { dataset: {}, style: {} };
    vm.runInNewContext(script, {
      document: { documentElement: root },
      localStorage: { getItem() { throw new Error("unavailable"); } },
      matchMedia: () => ({ matches: dark }),
    });
    assert.equal(root.style.backgroundColor, dark ? "#191a1c" : "#ffffff");
    assert.equal(root.dataset.theme, "system");
    assert.equal(root.lang, "sk");
  }
});
