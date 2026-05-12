/**
 * Domain types for the Team page surface. Locked in brief E3-a; the
 * TeammateCard composite (E3-b) and the gateway `/api/v1/team`
 * endpoint (sibling brief) read from this shape.
 */

export type WorkerStatus =
  | "working"
  | "waiting" // waiting on the user's approval
  | "idle"
  | "blocked"
  | "paused";

export interface WorkerRole {
  /** Short marketing-style line ("books showings", "answers customer chats"). */
  tagline: string;
  /** Vertical key — receptionist, sales, support, etc. */
  vertical: string;
}

export interface WorkerHeadline {
  /** Short, scannable result lines. E.g. "8 replies sent", "$340 made". */
  bullets: string[];
  /** Optional next-action sentence. E.g. "morning routine at 06:30". */
  tomorrow?: string;
}

export interface Worker {
  id: string;
  name: string;
  avatarUrl?: string;
  role: WorkerRole;
  status: WorkerStatus;
  headline?: WorkerHeadline;
  /** ISO timestamp of the worker's most recent activity. */
  lastActiveAt?: string;
  /** Surfaces as a count on the worker's status pill + nav badge. */
  pendingApprovalCount?: number;
}

/** All status values, in canonical order for filter dropdowns. */
export const WORKER_STATUSES: readonly WorkerStatus[] = [
  "working",
  "waiting",
  "idle",
  "blocked",
  "paused",
] as const;

/** Human label for each status (matches the brief's filter options). */
export const WORKER_STATUS_LABELS: Record<WorkerStatus, string> = {
  working: "Working",
  waiting: "Waiting on you",
  idle: "Idle",
  blocked: "Blocked",
  paused: "Paused",
};

/**
 * Sort priority: waiting first (urgent), then working, then idle/paused,
 * blocked last. Within group, more recently active wins.
 *
 * Lower number = higher priority (sorts earlier).
 */
const STATUS_SORT_PRIORITY: Record<WorkerStatus, number> = {
  waiting: 0,
  working: 1,
  idle: 2,
  paused: 2,
  blocked: 3,
};

export function compareWorkersForGrid(a: Worker, b: Worker): number {
  const pa = STATUS_SORT_PRIORITY[a.status];
  const pb = STATUS_SORT_PRIORITY[b.status];
  if (pa !== pb) return pa - pb;
  const ta = a.lastActiveAt ? Date.parse(a.lastActiveAt) : 0;
  const tb = b.lastActiveAt ? Date.parse(b.lastActiveAt) : 0;
  return tb - ta;
}
