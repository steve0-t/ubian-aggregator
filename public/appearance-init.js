// Blocking, bundled script: paint the saved base palette before the app loads.
// No account/session data belongs in this preference record.
(() => {
  let saved;
  try {
    saved = JSON.parse(localStorage.getItem("cestou.preferences.v1"));
  } catch {
    saved = null;
  }
  const theme = ["light", "dark", "black"].includes(saved?.theme)
    ? saved.theme
    : "system";
  const root = document.documentElement;
  root.dataset.theme = theme;
  root.lang = ["en", "uk"].includes(saved?.language) ? saved.language : "sk";
  const dark =
    theme === "dark" ||
    theme === "black" ||
    (theme === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  root.style.backgroundColor =
    theme === "black" ? "#000000" : dark ? "#191a1c" : "#ffffff";
  root.style.colorScheme = dark ? "dark" : "light";
})();
