import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { isNative, loadDeviceTheme, saveDeviceTheme } from "./lib/api/platform";
import { applyAppearance, readPreferences } from "./lib/preferences";

async function start() {
  const initialPreferences = readPreferences();
  let initialThemeSaved = true;
  if (isNative()) {
    try {
      const savedTheme = await loadDeviceTheme();
      if (savedTheme === null) await saveDeviceTheme(initialPreferences.theme);
      else initialPreferences.theme = savedTheme;
    } catch {
      initialThemeSaved = false;
    }
  }
  applyAppearance(initialPreferences);
  document.documentElement.lang = initialPreferences.language;

  mount(App, {
    target: document.getElementById("app")!,
    props: { initialPreferences, initialThemeSaved },
  });
}

void start();
