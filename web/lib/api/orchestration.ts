import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

export type OrchestrationStatus = "queued" | "running" | "waiting" | "blocked" | "done";
export type SlaState = "healthy" | "warning" | "breached";

export interface OrchestrationRun {
  id: string;
  worker: string;
  workerId: string;
  skill: string;
  status: OrchestrationStatus;
  slaState: SlaState;
  latencyMs: number;
  targetLatencyMs: number;
  retryAttempt: number;
  retryMax: number;
  retryBackoff: "none" | "exponential";
  queuedAt: string;
}

export interface OrchestrationSummary {
  total: number;
  running: number;
  queued: number;
  breached: number;
}

export interface OrchestrationResponse {
  runs: OrchestrationRun[];
  summary: OrchestrationSummary;
}

export interface OrchestrationFilters {
  worker: string;
  skill: string;
  status: "all" | OrchestrationStatus;
  slaState: "all" | SlaState;
}

/**
 * One node in a team-run tree: a `lead` row with the child delegation
 * rows it coordinated beneath it.
 *
 * The backend Supervisor engine persists each child delegation +
 * the lead's synthesis as `provider='orchestrator'` rows whose
 * `title` (surfaced here as {@link OrchestrationRun.skill}) carries
 * the only linkage signal the read endpoint exposes:
 *
 * - child step rows are titled `"[team] <stepId> → <task>"`
 * - the lead row is titled `"[team] lead synthesis — <plan title>"`
 *
 * `GET /api/v1/orchestration/runs` returns these flattened with **no
 * per-team-run id**, so {@link groupTeamRuns} reconstructs the
 * lead-with-children shape from that `[team]` prefix. See the gap note
 * on {@link groupTeamRuns}.
 */
export interface TeamRunGroup {
  /** Stable key for the group (the lead run id, or a synthetic key). */
  key: string;
  /** The lead / synthesis run, when one is present. */
  lead: OrchestrationRun | null;
  /** Plain-English title parsed from the lead row, if available. */
  title: string;
  /** Child delegation runs the lead coordinated. */
  children: OrchestrationRun[];
}

export interface TeamRunPartition {
  /** Team-run trees reconstructed from `[team]`-prefixed rows. */
  teams: TeamRunGroup[];
  /** Runs that are not part of any team run (the ordinary queue). */
  flat: OrchestrationRun[];
}

const TEAM_PREFIX = "[team] ";
const LEAD_MARKER = "[team] lead synthesis";

export type OrchestrationSource = "gateway" | "mock-offline" | "mock-bypass";

export interface FetchOrchestrationResult extends OrchestrationResponse {
  source: OrchestrationSource;
}

const WORKERS = ["Maya", "Nora", "Jules", "Anika", "Theo", "Priya"];
const SKILLS = [
  "invoice.extract",
  "vendor.followup",
  "approval.route",
  "candidate.screen",
  "deal.validate",
];
const STATUSES: OrchestrationStatus[] = ["running", "queued", "waiting", "blocked", "done"];

export const EMPTY_ORCHESTRATION_FILTERS: OrchestrationFilters = {
  worker: "",
  skill: "",
  status: "all",
  slaState: "all",
};

export const SAMPLE_ORCHESTRATION_RUNS: OrchestrationRun[] = Array.from(
  { length: 64 },
  (_, idx) => {
    const target = 90000 + (idx % 4) * 30000;
    const latency = target * (idx % 7 === 0 ? 1.35 : idx % 5 === 0 ? 0.92 : 0.44);
    const slaState: SlaState =
      latency > target ? "breached" : latency > target * 0.8 ? "warning" : "healthy";
    return {
      id: `run-${String(idx + 1).padStart(3, "0")}`,
      worker: WORKERS[idx % WORKERS.length],
      workerId: `worker-${idx % WORKERS.length}`,
      skill: SKILLS[idx % SKILLS.length],
      status: STATUSES[idx % STATUSES.length],
      slaState,
      latencyMs: Math.round(latency),
      targetLatencyMs: target,
      retryAttempt: idx % 4,
      retryMax: 3 + (idx % 2),
      retryBackoff: idx % 6 === 0 ? "none" : "exponential",
      queuedAt: new Date(Date.UTC(2026, 4, 14, 9, idx % 60)).toISOString(),
    };
  },
);

/**
 * Two illustrative team runs appended to the mock queue so the
 * `/orchestration` tree view has something to render on the
 * auth-bypass mock path (NEVER in demo-live — same gate as the rest
 * of this module). These mirror the real `[team]`-prefixed row shape
 * the Supervisor engine writes, so the grouping logic exercised here
 * is the identical code that runs against the live endpoint.
 */
export const SAMPLE_TEAM_RUNS: OrchestrationRun[] = [
  // Team run 1 — Mariah leads a competitor-analysis fan-out.
  {
    id: "team-run-a-lead",
    worker: "Mariah",
    workerId: "worker-lead-a",
    skill: "[team] lead synthesis — Competitor landscape analysis",
    status: "running",
    slaState: "healthy",
    latencyMs: 41000,
    targetLatencyMs: 120000,
    retryAttempt: 0,
    retryMax: 3,
    retryBackoff: "exponential",
    queuedAt: new Date(Date.UTC(2026, 4, 14, 9, 1)).toISOString(),
  },
  {
    id: "team-run-a-1",
    worker: "Nora",
    workerId: "worker-1",
    skill: "[team] survey → Survey the competitor landscape",
    status: "done",
    slaState: "healthy",
    latencyMs: 52000,
    targetLatencyMs: 120000,
    retryAttempt: 0,
    retryMax: 3,
    retryBackoff: "exponential",
    queuedAt: new Date(Date.UTC(2026, 4, 14, 9, 2)).toISOString(),
  },
  {
    id: "team-run-a-2",
    worker: "Jules",
    workerId: "worker-2",
    skill: "[team] pricing → Pull pricing pages for the top five rivals",
    status: "running",
    slaState: "warning",
    latencyMs: 99000,
    targetLatencyMs: 120000,
    retryAttempt: 1,
    retryMax: 3,
    retryBackoff: "exponential",
    queuedAt: new Date(Date.UTC(2026, 4, 14, 9, 3)).toISOString(),
  },
  {
    id: "team-run-a-3",
    worker: "Anika",
    workerId: "worker-3",
    skill: "[team] reviews → Summarise G2 + Capterra reviews",
    status: "queued",
    slaState: "healthy",
    latencyMs: 4000,
    targetLatencyMs: 120000,
    retryAttempt: 0,
    retryMax: 3,
    retryBackoff: "exponential",
    queuedAt: new Date(Date.UTC(2026, 4, 14, 9, 4)).toISOString(),
  },
  // Team run 2 — Patrick leads a quarterly-report roll-up.
  {
    id: "team-run-b-lead",
    worker: "Patrick",
    workerId: "worker-lead-b",
    skill: "[team] lead synthesis — Q2 status roll-up",
    status: "waiting",
    slaState: "healthy",
    latencyMs: 8000,
    targetLatencyMs: 120000,
    retryAttempt: 0,
    retryMax: 3,
    retryBackoff: "exponential",
    queuedAt: new Date(Date.UTC(2026, 4, 14, 9, 5)).toISOString(),
  },
  {
    id: "team-run-b-1",
    worker: "Priya",
    workerId: "worker-4",
    skill: "[team] revenue → Compile revenue numbers from finance",
    status: "blocked",
    slaState: "breached",
    latencyMs: 138000,
    targetLatencyMs: 120000,
    retryAttempt: 2,
    retryMax: 3,
    retryBackoff: "exponential",
    queuedAt: new Date(Date.UTC(2026, 4, 14, 9, 6)).toISOString(),
  },
  {
    id: "team-run-b-2",
    worker: "Theo",
    workerId: "worker-5",
    skill: "[team] product → Gather product-shipped highlights",
    status: "done",
    slaState: "healthy",
    latencyMs: 61000,
    targetLatencyMs: 120000,
    retryAttempt: 0,
    retryMax: 3,
    retryBackoff: "exponential",
    queuedAt: new Date(Date.UTC(2026, 4, 14, 9, 7)).toISOString(),
  },
];

/** `true` iff the run is part of a team run (a `[team]`-prefixed row). */
export function isTeamRun(run: OrchestrationRun): boolean {
  return run.skill.startsWith(TEAM_PREFIX);
}

/** `true` iff the run is a lead's synthesis row, not a child step. */
function isLeadRun(run: OrchestrationRun): boolean {
  return run.skill.startsWith(LEAD_MARKER);
}

/**
 * Strip the `[team] ` prefix and the `lead synthesis — ` / `<stepId> → `
 * decoration so the tree can show a clean human label.
 */
export function teamRunLabel(run: OrchestrationRun): string {
  const body = run.skill.slice(TEAM_PREFIX.length);
  if (run.skill.startsWith(LEAD_MARKER)) {
    const dash = body.indexOf("—");
    return dash >= 0 ? body.slice(dash + 1).trim() : body.replace(/^lead synthesis\b/, "").trim();
  }
  const arrow = body.indexOf("→");
  return arrow >= 0 ? body.slice(arrow + 1).trim() : body.trim();
}

/**
 * Partition runs into team-run trees + the ordinary flat queue.
 *
 * **Grouping signal & honest gap.** The read endpoint
 * (`GET /api/v1/orchestration/runs`) exposes no per-team-run id, only
 * the `[team]` title prefix the Supervisor engine writes. So we group
 * *all* `[team]`-prefixed rows: every `[team] lead synthesis` row
 * becomes a lead node, and the remaining `[team]` step rows are
 * attached as children. When exactly one lead is present, all step
 * rows hang under it (the common single-team demo case). When several
 * leads are present we cannot attribute a given step to a specific
 * lead from the wire shape alone, so the step rows are grouped into a
 * shared "Delegated steps" node rather than guessed — surfacing the
 * gap instead of faking precise per-lead edges. A real per-team-run id
 * on the row would let us draw exact edges; that is the documented
 * follow-up (add `teamRunId` to the run JSON + select it through).
 */
export function groupTeamRuns(runs: OrchestrationRun[]): TeamRunPartition {
  const flat: OrchestrationRun[] = [];
  const leads: OrchestrationRun[] = [];
  const steps: OrchestrationRun[] = [];
  for (const run of runs) {
    if (!isTeamRun(run)) {
      flat.push(run);
    } else if (isLeadRun(run)) {
      leads.push(run);
    } else {
      steps.push(run);
    }
  }
  if (leads.length === 0 && steps.length === 0) {
    return { teams: [], flat };
  }
  // Single lead: all step rows belong to it — draw a precise tree.
  if (leads.length <= 1) {
    const lead = leads[0] ?? null;
    return {
      teams: [
        {
          key: lead?.id ?? "team-unattributed",
          lead,
          title: lead ? teamRunLabel(lead) : "Delegated steps",
          children: steps,
        },
      ],
      flat,
    };
  }
  // Multiple leads: one node per lead (no children we can attribute
  // precisely) + one shared node holding the step rows. Honest about
  // the missing per-run linkage rather than guessing edges.
  const teams: TeamRunGroup[] = leads.map((lead) => ({
    key: lead.id,
    lead,
    title: teamRunLabel(lead),
    children: [],
  }));
  if (steps.length > 0) {
    teams.push({
      key: "team-delegated-steps",
      lead: null,
      title: "Delegated steps",
      children: steps,
    });
  }
  return { teams, flat };
}

function isStrictProduction(): boolean {
  return process.env.NODE_ENV === "production" && process.env.NEXT_PUBLIC_ENV === "production";
}

function isAuthBypassed(): boolean {
  return process.env.NODE_ENV !== "production" && process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1" && process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE !== "1";
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

export function summarizeRuns(runs: OrchestrationRun[]): OrchestrationSummary {
  return {
    total: runs.length,
    running: runs.filter((run) => run.status === "running").length,
    queued: runs.filter((run) => run.status === "queued").length,
    breached: runs.filter((run) => run.slaState === "breached").length,
  };
}

export function applyOrchestrationFilters(
  runs: OrchestrationRun[],
  filters: OrchestrationFilters,
): OrchestrationRun[] {
  return runs.filter((run) => {
    if (filters.worker && run.worker !== filters.worker) return false;
    if (filters.skill && run.skill !== filters.skill) return false;
    if (filters.status !== "all" && run.status !== filters.status) return false;
    if (filters.slaState !== "all" && run.slaState !== filters.slaState) return false;
    return true;
  });
}

/**
 * Mock queue surfaced on the auth-bypass / offline paths: the team
 * runs first (so the tree view has data) followed by the flat sample
 * queue. Only ever returned when {@link isAuthBypassed} (and never in
 * demo-live) or on a dev 404 — the gateway response is used verbatim
 * otherwise, so the tree on the live path is driven entirely by real
 * `[team]`-prefixed rows.
 */
const MOCK_QUEUE: OrchestrationRun[] = [...SAMPLE_TEAM_RUNS, ...SAMPLE_ORCHESTRATION_RUNS];

export async function fetchOrchestrationRuns(): Promise<FetchOrchestrationResult> {
  if (isAuthBypassed()) {
    return {
      runs: MOCK_QUEUE,
      summary: summarizeRuns(MOCK_QUEUE),
      source: "mock-bypass",
    };
  }
  try {
    const body = await apiGet<OrchestrationResponse>("/api/v1/orchestration/runs");
    return { ...body, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return {
        runs: MOCK_QUEUE,
        summary: summarizeRuns(MOCK_QUEUE),
        source: "mock-offline",
      };
    }
    throw err;
  }
}

export async function cancelRun(runId: string): Promise<void> {
  await apiPost(`/api/v1/orchestration/runs/${encodeURIComponent(runId)}/cancel`, {});
}

export async function resetRetry(runId: string): Promise<void> {
  await apiPost(`/api/v1/orchestration/runs/${encodeURIComponent(runId)}/reset-retry`, {});
}

export async function forceEscalate(runId: string): Promise<void> {
  await apiPost(`/api/v1/orchestration/runs/${encodeURIComponent(runId)}/force-escalate`, {});
}
