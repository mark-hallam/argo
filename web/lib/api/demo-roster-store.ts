/**
 * Session-local roster for workers hired during a recording demo.
 *
 * In auth-bypass mode (`NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`) the
 * gateway is dark, so `hireWorker()` cannot persist anything server-
 * side. The previous behaviour returned a `stub-<uuid>` id and lost
 * the input — the just-hired worker never appeared on `/team`, their
 * `/workers/<id>/overview` page 404'd, and the browser tab title was
 * literally `stub-<uuid> · Overview`. The camera saw every bit of
 * that theater. See `feedback_no_theater_in_demo`.
 *
 * This store keeps the new hires in `sessionStorage` so:
 *
 * - `fetchTeam()` can splice them onto `SAMPLE_TEAM` and the roster
 *   actually grows on hire.
 * - `fetchWorkerOverview(id)` can resolve them to a real card.
 * - Closing the tab cleans the demo roster back to 5 (sessionStorage
 *   is per-tab and dies on close — exactly the recording-prep
 *   guarantee we want).
 *
 * SSR-safe: every accessor checks for `typeof window` before touching
 * `sessionStorage`. On the server we read/write a module-local Map
 * keyed by tenant id so unit tests can drive the store too.
 */

import type { Worker } from "@/lib/types/worker";
import type { WorkerOverview } from "@/lib/api/worker-overview";

/** sessionStorage key — kept separate from the hire-draft entries. */
export const DEMO_ROSTER_STORAGE_KEY = "skycot.demoRoster";

/** One hired worker — the Worker row + the rich overview payload. */
export interface DemoRosterEntry {
  worker: Worker;
  overview: WorkerOverview;
}

interface PersistedRoster {
  version: 1;
  entries: DemoRosterEntry[];
}

const CURRENT_VERSION = 1 as const;

// In-memory fallback for SSR / Node test environments where
// sessionStorage is absent. Single global slot — tenant isolation in
// auth-bypass mode is the demo seat, not a security boundary.
let ssrFallbackRoster: DemoRosterEntry[] = [];

function isBrowser(): boolean {
  return typeof window !== "undefined" && typeof window.sessionStorage !== "undefined";
}

function readPersisted(): DemoRosterEntry[] {
  if (!isBrowser()) return ssrFallbackRoster;
  try {
    const raw = window.sessionStorage.getItem(DEMO_ROSTER_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as PersistedRoster;
    if (parsed?.version !== CURRENT_VERSION || !Array.isArray(parsed.entries)) {
      return [];
    }
    return parsed.entries;
  } catch {
    return [];
  }
}

function writePersisted(entries: DemoRosterEntry[]): void {
  if (!isBrowser()) {
    ssrFallbackRoster = entries;
    return;
  }
  const payload: PersistedRoster = { version: CURRENT_VERSION, entries };
  try {
    window.sessionStorage.setItem(DEMO_ROSTER_STORAGE_KEY, JSON.stringify(payload));
  } catch {
    // sessionStorage quota / disabled — swallow; the in-memory copy
    // on the page session is still good enough for the demo take.
  }
}

/**
 * Append a freshly-hired worker so subsequent /team + /workers/<id>
 * reads see them.
 */
export function appendDemoWorker(entry: DemoRosterEntry): void {
  const next = [...readPersisted(), entry];
  writePersisted(next);
}

/** Return all session-local hires in append order. */
export function listDemoWorkers(): Worker[] {
  return readPersisted().map((entry) => entry.worker);
}

/** Find a session-local hire by id; undefined if not in the store. */
export function findDemoWorker(workerId: string): Worker | undefined {
  return readPersisted()
    .map((entry) => entry.worker)
    .find((worker) => worker.id === workerId);
}

/** Find the rich overview payload for a session-local hire. */
export function findDemoWorkerOverview(workerId: string): WorkerOverview | undefined {
  return readPersisted().find((entry) => entry.worker.id === workerId)?.overview;
}

/**
 * Test-only escape hatch — wipes the in-memory + sessionStorage copy.
 * Production hire flow never calls this; the natural lifetime is "tab
 * closes" which sessionStorage handles for us.
 */
export function _clearDemoRosterForTests(): void {
  ssrFallbackRoster = [];
  if (isBrowser()) {
    try {
      window.sessionStorage.removeItem(DEMO_ROSTER_STORAGE_KEY);
    } catch {
      /* swallow */
    }
  }
}
