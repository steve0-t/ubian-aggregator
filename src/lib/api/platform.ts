export type Destination = "journeys" | "login" | "support";

export interface PlatformStatus {
  android: boolean;
  policyVersion: number;
  optionalRuleCount: number;
  providerState: "closed" | "opening" | "open" | "cleaning" | "cleanup_failed";
  lastError: string | null;
}

export interface ProbeReport {
  roundTrip: boolean;
  tamperRejected: boolean;
  uniqueNonces: boolean;
  missingKeyRejected: boolean;
  hardwareBacked: boolean;
}

declare global {
  interface Window {
    __TAURI__?: {
      core: {
        invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
      };
    };
  }
}

export function isNative(): boolean {
  return typeof window.__TAURI__?.core.invoke === "function";
}

// These are the only frontend actions. The global API is injected by Tauri into
// bundled assets only; provider pages run in a different, bridge-free WebView.
function invoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!window.__TAURI__) return Promise.reject(new Error("android_required"));
  return window.__TAURI__.core.invoke<T>(
    `plugin:android-platform|${command}`,
    args,
  );
}

export const platformStatus = () => invoke<PlatformStatus>("status");
export const openProvider = (destination: Destination, language: "sk" | "en") =>
  invoke<void>("open_provider", { destination, language });
export const probeKeystore = () => invoke<ProbeReport>("keystore_probe");
