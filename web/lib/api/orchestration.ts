import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { hasRealSession } from "@/lib/skycot/demo-mode";
import { isMissingOrUnreachable } from "./unreachable";

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
 * the only linkage signal the read endpoint exposes. Each row title
 * embeds the run-wide team id the engine mints once per run:
 *
 * - child step rows are titled `"[team:<runId>] <stepId> → <task>"`
 * - the lead row is titled `"[team:<runId>] lead synthesis — <plan title>"`
 *
 * `GET /api/v1/orchestration/runs` returns these flattened with no
 * dedicated id column, so {@link groupTeamRuns} reconstructs the
 * lead-with-children shape by parsing `<runId>` off the prefix —
 * distinct ids ⇒ distinct trees, even for concurrent runs. Rows
 * written before the id existed carry a bare `"[team] …"` prefix and
 * fall back to a shared node (see {@link groupTeamRuns}).
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

/**
 * Matches the `[team]` / `[team:<id>]` prefix a Supervisor row title
 * carries. Group 1 is the team-run id when present (the `[team:<id>]`
 * form the engine writes today); absent for the legacy bare `[team]`
 * form so old rows still render. The trailing space is consumed so
 * {@link teamRunLabel} sees only the decorated body.
 */
const TEAM_PREFIX_RE = /^\[team(?::([^\]]+))?\] /;
const LEAD_BODY_MARKER = "lead synthesis";

/**
 * Parse the `[team]` / `[team:<id>]` prefix off a run's title. Returns
 * `null` for non-team rows; otherwise the team-run id (or `null` for
 * the legacy bare prefix) plus the remaining body text.
 */
function parseTeamPrefix(run: OrchestrationRun): { teamRunId: string | null; body: string } | null {
  const match = TEAM_PREFIX_RE.exec(run.skill);
  if (!match) return null;
  return { teamRunId: match[1] ?? null, body: run.skill.slice(match[0].length) };
}

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
 * of this module). These mirror the real `[team:<id>]`-prefixed row
 * shape the Supervisor engine writes (each run minting its own id), so
 * the grouping logic exercised here is the identical code that runs
 * against the live endpoint — two distinct ids ⇒ two distinct trees.
 */
export const SAMPLE_TEAM_RUNS: OrchestrationRun[] = [
  // Team run 1 — Mariah leads a competitor-analysis fan-out.
  {
    id: "team-run-a-lead",
    worker: "Mariah",
    workerId: "worker-lead-a",
    skill: "[team:team-run-a] lead synthesis — Competitor landscape analysis",
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
    skill: "[team:team-run-a] survey → Survey the competitor landscape",
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
    skill: "[team:team-run-a] pricing → Pull pricing pages for the top five rivals",
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
    skill: "[team:team-run-a] reviews → Summarise G2 + Capterra reviews",
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
    skill: "[team:team-run-b] lead synthesis — Q2 status roll-up",
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
    skill: "[team:team-run-b] revenue → Compile revenue numbers from finance",
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
    skill: "[team:team-run-b] product → Gather product-shipped highlights",
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

/** `true` iff the run is part of a team run (a `[team]`/`[team:<id>]` row). */
export function isTeamRun(run: OrchestrationRun): boolean {
  return TEAM_PREFIX_RE.test(run.skill);
}

/** `true` iff the run is a lead's synthesis row, not a child step. */
function isLeadRun(run: OrchestrationRun): boolean {
  const parsed = parseTeamPrefix(run);
  return parsed !== null && parsed.body.startsWith(LEAD_BODY_MARKER);
}

/**
 * Strip the `[team]` / `[team:<id>]` prefix and the
 * `lead synthesis — ` / `<stepId> → ` decoration so the tree can show
 * a clean human label.
 */
export function teamRunLabel(run: OrchestrationRun): string {
  const parsed = parseTeamPrefix(run);
  if (!parsed) return run.skill;
  const { body } = parsed;
  if (body.startsWith(LEAD_BODY_MARKER)) {
    const dash = body.indexOf("—");
    return dash >= 0 ? body.slice(dash + 1).trim() : body.replace(/^lead synthesis\b/, "").trim();
  }
  const arrow = body.indexOf("→");
  return arrow >= 0 ? body.slice(arrow + 1).trim() : body.trim();
}

/** Build one tree node from a group's lead row (if any) + its steps. */
function makeGroup(key: string, lead: OrchestrationRun | null, steps: OrchestrationRun[]): TeamRunGroup {
  return {
    key,
    lead,
    title: lead ? teamRunLabel(lead) : "Delegated steps",
    children: steps,
  };
}

/**
 * Partition runs into team-run trees + the ordinary flat queue.
 *
 * **Grouping signal.** The Supervisor engine mints one run-wide id per
 * team run and embeds it in every persisted row title as a
 * `[team:<id>]` prefix (the read endpoint
 * `GET /api/v1/orchestration/runs` exposes no dedicated id column, so
 * the title carries it). We bucket `[team:<id>]`-prefixed rows by that
 * `<id>`: within a bucket the `lead synthesis` row is the lead and the
 * `→` rows are its children, so **two distinct ids become two
 * distinct trees** — concurrent team runs no longer collide.
 *
 * **Legacy fallback.** Rows written before the id existed carry a bare
 * `[team]` prefix (no id). Those keep the older heuristic: a single
 * bare lead gets a precise tree, multiple bare leads fall back to a
 * shared "Delegated steps" node rather than guessing edges. New rows
 * never hit this path.
 */
export function groupTeamRuns(runs: OrchestrationRun[]): TeamRunPartition {
  const flat: OrchestrationRun[] = [];
  // Identified rows bucket by their team-run id, preserving first-seen
  // bucket order (Map iteration order) for a stable wire shape.
  const byId = new Map<string, { lead: OrchestrationRun | null; steps: OrchestrationRun[] }>();
  // Legacy bare-`[team]` rows (no id) handled separately below.
  const legacyLeads: OrchestrationRun[] = [];
  const legacySteps: OrchestrationRun[] = [];

  for (const run of runs) {
    const parsed = parseTeamPrefix(run);
    if (!parsed) {
      flat.push(run);
      continue;
    }
    const isLead = parsed.body.startsWith(LEAD_BODY_MARKER);
    if (parsed.teamRunId === null) {
      // Legacy: no id to attribute by.
      if (isLead) legacyLeads.push(run);
      else legacySteps.push(run);
      continue;
    }
    const bucket = byId.get(parsed.teamRunId) ?? { lead: null, steps: [] };
    if (isLead) bucket.lead = run;
    else bucket.steps.push(run);
    byId.set(parsed.teamRunId, bucket);
  }

  const teams: TeamRunGroup[] = [];
  // One precise tree per distinct team-run id.
  for (const [id, bucket] of byId) {
    teams.push(makeGroup(bucket.lead?.id ?? `team:${id}`, bucket.lead, bucket.steps));
  }
  // Legacy fallback for bare-`[team]` rows (no id on the wire).
  appendLegacyGroups(teams, legacyLeads, legacySteps);

  return { teams, flat };
}

/**
 * Append tree nodes for legacy bare-`[team]` rows (no id): a single
 * lead gets its steps as a precise tree; multiple leads each get a
 * childless node plus one shared "Delegated steps" node for the
 * un-attributable steps. Mutates `teams` in place.
 */
function appendLegacyGroups(
  teams: TeamRunGroup[],
  leads: OrchestrationRun[],
  steps: OrchestrationRun[],
): void {
  if (leads.length === 0 && steps.length === 0) return;
  if (leads.length <= 1) {
    const lead = leads[0] ?? null;
    teams.push(makeGroup(lead?.id ?? "team-unattributed", lead, steps));
    return;
  }
  for (const lead of leads) teams.push(makeGroup(lead.id, lead, []));
  if (steps.length > 0) teams.push(makeGroup("team-delegated-steps", null, steps));
}

function isStrictProduction(): boolean {
  return process.env.NODE_ENV === "production" && process.env.NEXT_PUBLIC_ENV === "production";
}

function isAuthBypassed(): boolean {
  // Per-user gate (sprint, 2026-06-07, mock-leak class): a REAL
  // signed-in user (the `skycot-session=1` marker cookie is present) is
  // NEVER served the auth-bypass mock fixtures, even in a demo-flagged
  // deploy. The bypass short-circuit is for ANONYMOUS demo visitors only.
  if (hasRealSession()) return false;
  return process.env.NODE_ENV !== "production" && process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1" && process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE !== "1";
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
