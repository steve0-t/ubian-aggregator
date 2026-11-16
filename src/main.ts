import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { applyAppearance, readPreferences } from "./lib/preferences";

// Apply saved colors before mounting so the interface starts in the chosen theme.
const initialPreferences = readPreferences();
applyAppearance(initialPreferences);
document.documentElement.lang = initialPreferences.language;

mount(App, {
  target: document.getElementById("app")!,
  props: { initialPreferences },
});
