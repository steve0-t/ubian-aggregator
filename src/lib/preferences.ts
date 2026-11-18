import type { Language } from "./i18n";

export type Theme = "system" | "light" | "dark" | "black";

export interface Preferences {
  accent: string;
  theme: Theme;
  language: Language;
}

type PreferenceStorage = Pick<Storage, "getItem" | "setItem">;

// Keep the storage key stable so renaming the app preserves saved appearance.
export const PREFERENCES_KEY = "cestou.preferences.v1";
export const DEFAULT_ACCENT = "#008837";
export const DEFAULT_PREFERENCES: Readonly<Preferences> = {
  accent: DEFAULT_ACCENT,
  theme: "system",
  language: "sk",
};

export const ACCENT_PRESETS = [
  { name: "green", color: DEFAULT_ACCENT },
  { name: "teal", color: "#00857d" },
  { name: "blue", color: "#2563eb" },
  { name: "purple", color: "#8055d9" },
  { name: "rose", color: "#cf386f" },
  { name: "orange", color: "#c15e0d" },
] as const;

export function isAccent(value: unknown): value is string {
  return typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value);
}

export function readPreferences(storage?: PreferenceStorage): Preferences {
  try {
    const saved: unknown = JSON.parse(
      (storage ?? window.localStorage).getItem(PREFERENCES_KEY) ?? "null",
    );
    if (!saved || typeof saved !== "object" || Array.isArray(saved)) {
      return { ...DEFAULT_PREFERENCES };
    }
    const value = saved as Record<string, unknown>;
    return {
      accent: isAccent(value.accent)
        ? value.accent.toLowerCase()
        : DEFAULT_ACCENT,
      theme:
        value.theme === "light" ||
        value.theme === "dark" ||
        value.theme === "black"
          ? value.theme
          : "system",
      language:
        value.language === "en" || value.language === "uk"
          ? value.language
          : "sk",
    };
  } catch {
    return { ...DEFAULT_PREFERENCES };
  }
}

export function savePreferences(
  preferences: Preferences,
  storage?: PreferenceStorage,
): boolean {
  try {
    (storage ?? window.localStorage).setItem(
      PREFERENCES_KEY,
      JSON.stringify(preferences),
    );
    return true;
  } catch {
    // Appearance remains usable when browser storage is unavailable or full.
    return false;
  }
}

function channels(hex: string): number[] {
  return [1, 3, 5].map((offset) => parseInt(hex.slice(offset, offset + 2), 16));
}

function luminance(hex: string): number {
  const linear = channels(hex).map((channel) => {
    const value = channel / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
}

function contrast(first: string, second: string): number {
  const a = luminance(first);
  const b = luminance(second);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

function mix(first: string, second: string, amount: number): string {
  const target = channels(second);
  return `#${channels(first)
    .map((channel, index) =>
      Math.round(channel + (target[index] - channel) * amount)
        .toString(16)
        .padStart(2, "0"),
    )
    .join("")}`;
}

function readableAccent(color: string, backgrounds: string[]): string {
  const target = luminance(backgrounds[0]) > 0.5 ? "#000000" : "#ffffff";
  for (let step = 0; step <= 100; step++) {
    const candidate = mix(color, target, step / 100);
    if (
      backgrounds.every((background) => contrast(candidate, background) >= 5)
    ) {
      return candidate;
    }
  }
  return target;
}

export function accentTokens(value: string): Record<string, string> {
  const color = isAccent(value) ? value.toLowerCase() : DEFAULT_ACCENT;
  const lightSoft = mix(color, "#ffffff", 0.93);
  const darkSoft = mix(color, "#191a1c", 0.84);
  const blackSoft = mix(color, "#000000", 0.86);
  return {
    "--accent": color,
    "--on-accent":
      contrast(color, "#ffffff") >= contrast(color, "#000000")
        ? "#ffffff"
        : "#000000",
    "--accent-light-text": readableAccent(color, ["#f7f8f8", lightSoft]),
    "--accent-dark-text": readableAccent(color, ["#2a2b2e", darkSoft]),
    "--accent-black-text": readableAccent(color, ["#1b1b1b", blackSoft]),
    "--accent-light-soft": lightSoft,
    "--accent-dark-soft": darkSoft,
    "--accent-black-soft": blackSoft,
    "--accent-light-border": mix(color, "#ffffff", 0.72),
    "--accent-dark-border": mix(color, "#191a1c", 0.52),
    "--accent-black-border": mix(color, "#000000", 0.45),
  };
}

export function applyAppearance(
  preferences: Pick<Preferences, "accent" | "theme">,
) {
  document.documentElement.dataset.theme = preferences.theme;
  // The blocking startup script supplies a background while bundled CSS loads.
  document.documentElement.style.removeProperty("background-color");
  document.documentElement.style.removeProperty("color-scheme");
  for (const [property, value] of Object.entries(
    accentTokens(preferences.accent),
  )) {
    document.documentElement.style.setProperty(property, value);
  }
}
