/**
 * Domain types for the Worker primitive — a single AI teammate
 * inside a tenant. Originally locked by brief E3-a (Team page +
 * `/api/v1/team` endpoint); extended by brief E4-a with the
 * org-hierarchy + per-row KPI + per-role permission fields that
 * the worker-profile surface (E4-a..i) reads.
 *
 * v1 posture: department / team / kpiToday / permissions are all
 * optional so E3-a's leaner shape stays backwards-compatible. The
 * Track I `mission-enterprise-org-hierarchy` brief promotes
 * Department + Team into first-class entities and derives the
 * approval-routing chain from the org tree.
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

/** Department or team — denormalised onto Worker in v1. */
export interface WorkerOrgUnit {
  id: string;
  name: string;
}

export interface WorkerKpiToday {
  /** e.g. "interviews booked", "tickets resolved", "$ approved". */
  label: string;
  /** e.g. "3", "12", "$340" — display-ready string, not numeric. */
  value: string;
}

/**
 * Per-role capability flags. v1 surfaces all delegators with full
 * permissions; downstream tabs (E4-c/E4-f) gate field-level edits
 * against these flags. The Track H
 * `mission-enterprise-worker-profile-rbac-tabs` brief turns this
 * into RBAC-aware tab visibility at the page-shell level.
 */
export interface WorkerPermissions {
  canEditCharter: boolean;
  canEditDuties: boolean;
  canPauseResume: boolean;
  canArchive: boolean;
  /** Operator-only in the enterprise target. */
  canDelete: boolean;
  canViewAudit: boolean;
  /** Operator-only in the enterprise target. */
  canViewSettings: boolean;
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

  // --- E4-a additions (all optional for backwards compat with E3-a) ---
  department?: WorkerOrgUnit;
  team?: WorkerOrgUnit;
  kpiToday?: WorkerKpiToday;
  permissions?: WorkerPermissions;
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

/** Compact label used on the WorkerHero status pill (no "on you" tail). */
export const WORKER_STATUS_PILL_LABEL: Record<WorkerStatus, string> = {
  working: "Working",
  waiting: "Waiting",
  idle: "Idle",
  blocked: "Blocked",
  paused: "Paused",
};

/** Display tone for the status pill (matches `StatusPill` tone prop). */
export const WORKER_STATUS_TONE: Record<
  WorkerStatus,
  "info" | "warning" | "muted" | "error"
> = {
  working: "info",
  waiting: "warning",
  idle: "muted",
  blocked: "error",
  paused: "muted",
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

/** Default permissions for mock data; production hydrates from gateway RBAC. */
export const DEFAULT_DELEGATOR_PERMISSIONS: WorkerPermissions = {
  canEditCharter: true,
  canEditDuties: true,
  canPauseResume: true,
  canArchive: true,
  canDelete: false,
  canViewAudit: true,
  canViewSettings: false,
};
