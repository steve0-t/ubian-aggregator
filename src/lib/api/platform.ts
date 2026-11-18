import type { Language } from "../i18n";

export type Destination = "login" | "support";

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
  if (!window.__TAURI__) return Promise.reject("android_required");
  return window.__TAURI__.core.invoke<T>(
    `plugin:android-platform|${command}`,
    args,
  );
}

export const platformStatus = () => invoke<PlatformStatus>("status");
export const openProvider = (destination: Destination, language: Language) =>
  invoke<void>("open_provider", { destination, language });
export const probeKeystore = () => invoke<ProbeReport>("keystore_probe");
export const setSystemAppearance = (theme: "light" | "dark" | "black") =>
  invoke<void>("set_appearance", { theme });

export interface Stop {
  id: number;
  name: string;
}

export type Transport = "all" | "bus" | "train" | "urban";
export type TimeOccurrence = "earlier" | "later";
export interface JourneyQuery {
  from: Stop;
  to: Stop;
  date: string;
  time: string;
  arriveBy: boolean;
  directOnly: boolean;
  transport: Transport;
  timeOccurrence: TimeOccurrence;
}
export interface DepartureQuery {
  stop: Stop;
  date: string;
  time: string;
  transport: Transport;
  timeOccurrence: TimeOccurrence;
}
export interface Service {
  line: string;
  name: string;
  operator: string;
  destination: string;
  cancelled: boolean;
}
export interface Leg {
  from: Stop;
  to: Stop;
  departure: string;
  arrival: string;
  fromPlatform: string | null;
  toPlatform: string | null;
  walkingMeters: number;
  service: Service | null;
  detailToken: string | null;
}
export interface Journey {
  id: string;
  departure: string;
  arrival: string;
  durationMinutes: number;
  transfers: number;
  legs: Leg[];
}
export interface Departure {
  id: string;
  stop: Stop;
  departure: string;
  platform: string | null;
  service: Service;
  detailToken: string | null;
}
export interface DetailStop {
  stop: Stop;
  departure: string;
  platform: string | null;
}
export interface ConnectionDetail {
  stops: DetailStop[];
}

export const lookupStops = (query: string, requestId: string) =>
  invoke<Stop[]>("lookup_stops", { requestId, request: { query } });
export const searchJourneys = (request: JourneyQuery, requestId: string) =>
  invoke<Journey[]>("search_journeys", { requestId, request });
export const stopDepartures = (request: DepartureQuery, requestId: string) =>
  invoke<Departure[]>("departures", { requestId, request });
export const connectionDetail = (token: string, requestId: string) =>
  invoke<ConnectionDetail>("connection_detail", {
    requestId,
    request: { token },
  });
export const cancelRequest = (requestId: string) =>
  invoke<void>("cancel_request", { requestId });

export type AccountState =
  "signed_out" | "signing_in" | "signed_in" | "expired" | "cached";
export interface AccountProfile {
  email: string;
  displayName: string | null;
}
export interface AccountSnapshot {
  state: AccountState;
  profile: AccountProfile | null;
  lastUpdated: string | null;
  storage: {
    availability:
      | "available"
      | "authentication_unavailable"
      | "unsupported_platform"
      | "key_missing"
      | "key_invalidated"
      | "invalid_record"
      | "io_failure";
    recordPresent: boolean;
  };
}
export const accountStatus = () => invoke<AccountSnapshot>("account_status");
export const accountLogin = (
  email: string,
  password: string,
  rememberSession: boolean,
  requestId: string,
) =>
  invoke<AccountSnapshot>("account_login", {
    requestId,
    request: { email, password, rememberSession },
  });
export const accountRefresh = (requestId: string) =>
  invoke<AccountSnapshot>("account_refresh", { requestId });
export const accountLogout = (requestId: string) =>
  invoke<{ account: AccountSnapshot; serverConfirmed: boolean }>(
    "account_logout",
    { requestId },
  );
export const accountForget = (requestId: string) =>
  invoke<AccountSnapshot>("account_forget", { requestId });
export const accountCancel = (requestId: string) =>
  invoke<void>("account_cancel", { requestId });

export const accountSaveSession = (requestId: string, language: Language) =>
  invoke<AccountSnapshot>("account_save_session", { requestId, language });
export const accountUnlockSession = (requestId: string, language: Language) =>
  invoke<AccountSnapshot>("account_unlock_session", { requestId, language });

// Await each native acknowledgement so rapid tab changes cannot let an older
// disable operation complete after the next account view enables protection.
let accountProtectionQueue: Promise<void> = Promise.resolve();
export function setAccountProtection(enabled: boolean): Promise<void> {
  const action = accountProtectionQueue
    .catch(() => {})
    .then(() => invoke<void>("account_protection", { enabled }));
  accountProtectionQueue = action;
  return action;
}
