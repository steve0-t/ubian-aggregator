export type SearchError =
  | "queryInvalid"
  | "invalidTime"
  | "offline"
  | "timeout"
  | "rateLimited"
  | "providerChanged"
  | "providerUnavailable"
  | "detailUnavailable"
  | "genericError"
  | "cancelled"
  | "nativeUnavailable";

export function searchError(error: unknown): SearchError {
  switch (error) {
    case "source_unavailable":
      return "providerUnavailable";
    case "invalid_query":
      return "queryInvalid";
    case "invalid_time":
    case "ambiguous_time":
      return "invalidTime";
    case "offline":
      return "offline";
    case "timeout":
      return "timeout";
    case "rate_limited":
      return "rateLimited";
    case "provider_changed":
      return "providerChanged";
    case "provider_unavailable":
      return "providerUnavailable";
    case "detail_unavailable":
      return "detailUnavailable";
    case "cancelled":
      return "cancelled";
    case "android_required":
      return "nativeUnavailable";
    default:
      return "genericError";
  }
}

let requestSequence = 0;
// Each screen owns one generation. Superseding or canceling aborts native work
// and invalidates its completion, including races with an IPC cancellation.
export class SearchRequest {
  private generation = 0;
  private activeId: string | null = null;
  private ids = new Map<number, string>();
  private abort: ((id: string) => Promise<unknown>) | undefined;
  constructor(abort?: (id: string) => Promise<unknown>) {
    this.abort = abort;
  }
  begin(): number {
    this.cancel();
    const ticket = ++this.generation;
    const id = `timetable-${Date.now()}-${++requestSequence}`;
    this.activeId = id;
    this.ids.clear();
    this.ids.set(ticket, id);
    return ticket;
  }
  requestId(ticket: number): string {
    return this.ids.get(ticket) ?? "";
  }
  cancel(): void {
    const id = this.activeId;
    this.activeId = null;
    this.ids.clear();
    this.generation++;
    if (id && this.abort) void this.abort(id).catch(() => {});
  }
  current(ticket: number): boolean {
    return ticket === this.generation;
  }
  finish(ticket: number): void {
    if (this.current(ticket)) this.activeId = null;
  }
}

export function validStopQuery(query: string): boolean {
  const length = [...query.trim()].length;
  return length >= 2 && length <= 100 && !/[\p{Cc}]/u.test(query);
}

export function timetableInput(instant = new Date()): {
  date: string;
  time: string;
} {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: "Europe/Bratislava",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).formatToParts(instant);
  const part = (type: string) =>
    parts.find((item) => item.type === type)?.value ?? "";
  return {
    date: `${part("year")}-${part("month")}-${part("day")}`,
    time: `${part("hour")}:${part("minute")}`,
  };
}
