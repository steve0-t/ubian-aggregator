export type SearchError =
  | "timetableBlocked" | "queryInvalid" | "offline" | "timeout"
  | "rateLimited" | "providerChanged" | "genericError" | "cancelled"
  | "nativeUnavailable";

export function searchError(error: unknown): SearchError {
  switch (error) {
    case "source_unavailable": return "timetableBlocked";
    case "invalid_query": return "queryInvalid";
    case "offline": return "offline";
    case "timeout": return "timeout";
    case "rate_limited": return "rateLimited";
    case "provider_changed": return "providerChanged";
    case "android_required": return "nativeUnavailable";
    default: return "genericError";
  }
}

// A canceled or superseded request cannot repopulate the local screen. No
// searches are written to storage. Network cancellation is still a source gate.
export class SearchRequest {
  private generation = 0;
  begin(): number { return ++this.generation; }
  cancel(): void { this.generation++; }
  current(ticket: number): boolean { return ticket === this.generation; }
}

export function validStopQuery(query: string): boolean {
  const length = [...query.trim()].length;
  return length >= 2 && length <= 100 && !/[\p{Cc}]/u.test(query);
}
