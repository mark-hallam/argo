/**
 * Workflows API client (Stream `web-workflows-live-data-binding`).
 *
 * Wires the `/workflows` catalog + `/workflows/<v>/<n>` visual viewer
 * to the gateway CRUD routes shipped in PR #1860:
 *
 *   GET /api/v1/verticals/<v>/workflows                → catalog per vertical
 *   GET /api/v1/verticals/<v>/workflows/<n>            → workflow head
 *   GET /api/v1/verticals/<v>/workflows/<n>/versions   → version list (#1980)
 *   POST /api/v1/verticals/<v>/workflows/<n>/rollback  → rollback (#1980)
 *
 * The gateway's `/verticals` discovery endpoint lives at the root
 * path (not under `/api/v1/`) and the CORS preflight handler only
 * matches `/api/v1/*` — so signed-in browser calls to `/verticals`
 * fail the preflight (404 on OPTIONS) before the actual GET ever
 * fires. To keep `/workflows` rendering in the browser we skip the
 * discovery round-trip entirely and derive the vertical list from
 * the local seed in `lib/sample-workflows`. Every canonical vertical
 * the catalog cares about is already represented there, and the
 * gateway-side reconcile (CORS-mounting `/api/v1/verticals` or
 * adding a flat `/api/v1/workflows` list) is a separate brief.
 *
 * `fetchAllWorkflows` fans the per-vertical call out in parallel and
 * folds the results into a single summary list. Per-vertical errors
 * (auth 401/403, 404, 5xx, network) are swallowed to an empty bucket
 * so one missing/forbidden vertical does not blank the whole
 * catalog. When every bucket comes back empty the function falls
 * through to the local seed, marked `mock-offline`.
 *
 * Strict prod (`NODE_ENV=production` + `NEXT_PUBLIC_ENV=production`)
 * propagates 5xx; the dev fall-through never fires.
 *
 * `parseStepsFromToml` is a minimal regex-based decoder for the
 * `workflow.toml` schema we own (header + `[workflow]` + `[[step]]`).
 * No new dependency; full TOML parsing is overkill for the four keys
 * we need (workflow.description, step.name/kind/description).
 *
 * `listVersions` + `rollbackToVersion` are the canonical
 * version-history + rollback bindings for the viewer UI button
 * (Stream `web-workflow-rollback-ui-button`). They delegate to the
 * gateway endpoints first wired by #1980; this module is the
 * canonical client home so the viewer page never has to reach into a
 * sibling file. The older `workflow-rollback.ts` module still exists
 * for backward compatibility and is not edited here.
 */
import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import {
  SAMPLE_WORKFLOW_SPECS,
  SAMPLE_WORKFLOW_SUMMARIES,
  type WorkflowRisk,
  type WorkflowSpec,
  type WorkflowStep,
  type WorkflowStepKind,
  type WorkflowSummary,
} from "@/lib/sample-workflows";

export type WorkflowDataSource = "gateway" | "mock-offline" | "mock-bypass";

export interface FetchAllWorkflowsResult {
  workflows: WorkflowSummary[];
  source: WorkflowDataSource;
}

export interface FetchWorkflowResult {
  workflow: WorkflowSpec | null;
  source: WorkflowDataSource;
}

interface WorkflowListEnvelope {
  vertical_id?: string;
  count?: number;
  items?: GatewayWorkflowItem[];
}

export interface GatewayWorkflowItem {
  id: string;
  tenant_id?: string;
  vertical_id: string;
  name: string;
  version: number;
  parent_id?: string | null;
  lifecycle_state?: string;
  created_at?: string;
  modified_by?: string;
  definition_toml?: string;
}

const VALID_STEP_KINDS: ReadonlySet<WorkflowStepKind> = new Set([
  "trigger",
  "skill_invoke",
  "approval",
  "tool_call",
  "notify",
  "decision",
  "data",
]);

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" && process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  return (
    process.env.NODE_ENV !== "production" && process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isRecoverable(err: unknown): boolean {
  return err instanceof ApiError || err instanceof TypeError;
}

/**
 * Unique vertical IDs the catalog should fan out to. Derived from the
 * local seed so the canonical list lives in one place — adding a new
 * sample workflow under a fresh vertical automatically widens the
 * gateway query without a second edit here.
 */
function discoverVerticalIds(): string[] {
  const seen = new Set<string>();
  for (const wf of SAMPLE_WORKFLOW_SUMMARIES) {
    if (wf.vertical) seen.add(wf.vertical);
  }
  return Array.from(seen);
}

export async function fetchAllWorkflows(): Promise<FetchAllWorkflowsResult> {
  if (isAuthBypassed()) {
    return { workflows: SAMPLE_WORKFLOW_SUMMARIES, source: "mock-bypass" };
  }
  const verticals = discoverVerticalIds();
  if (verticals.length === 0) {
    return { workflows: SAMPLE_WORKFLOW_SUMMARIES, source: "mock-offline" };
  }
  const buckets = await Promise.all(
    verticals.map(async (id): Promise<GatewayWorkflowItem[]> => {
      const path = `/api/v1/verticals/${encodeURIComponent(id)}/workflows`;
      try {
        const list = await apiGet<WorkflowListEnvelope>(path);
        return list.items ?? [];
      } catch (err) {
        // In dev / demo, any per-vertical failure (auth 401/403, 404,
        // 5xx, network) becomes an empty bucket so one missing or
        // forbidden vertical never blanks the whole catalog. Strict
        // prod still propagates so a broken deploy fails loudly
        // instead of masking the regression with seed data.
        if (!isStrictProduction() && isRecoverable(err)) return [];
        throw err;
      }
    }),
  );
  const items = buckets.flat();
  if (items.length === 0) {
    return { workflows: SAMPLE_WORKFLOW_SUMMARIES, source: "mock-offline" };
  }
  return {
    workflows: items.map(summaryFromGatewayItem),
    source: "gateway",
  };
}

export async function fetchWorkflow(
  verticalId: string,
  name: string,
): Promise<FetchWorkflowResult> {
  const key = `${verticalId}/${name}`;
  if (isAuthBypassed()) {
    return {
      workflow: SAMPLE_WORKFLOW_SPECS[key] ?? null,
      source: "mock-bypass",
    };
  }
  const path = `/api/v1/verticals/${encodeURIComponent(verticalId)}/workflows/${encodeURIComponent(name)}`;
  try {
    const item = await apiGet<GatewayWorkflowItem>(path);
    return { workflow: specFromGatewayItem(item), source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isRecoverable(err)) {
      return {
        workflow: SAMPLE_WORKFLOW_SPECS[key] ?? null,
        source: "mock-offline",
      };
    }
    throw err;
  }
}

export function summaryFromGatewayItem(item: GatewayWorkflowItem): WorkflowSummary {
  const parsed = parseStepsFromToml(item.definition_toml ?? "");
  const description = parsed.description || `Workflow ${item.name} in ${item.vertical_id}.`;
  return {
    name: item.name,
    vertical: item.vertical_id,
    description,
    steps: parsed.steps.length,
    versions: Math.max(1, Number(item.version) || 1),
    lastEdited: item.created_at ?? "—",
    lastEditor: item.modified_by ?? "—",
    risk: parsed.risk,
  };
}

export function specFromGatewayItem(item: GatewayWorkflowItem): WorkflowSpec {
  const parsed = parseStepsFromToml(item.definition_toml ?? "");
  return {
    name: item.name,
    vertical: item.vertical_id,
    description: parsed.description || `Workflow ${item.name} in ${item.vertical_id}.`,
    version: `v${Math.max(1, Number(item.version) || 1)}`,
    steps: parsed.steps,
    risk: parsed.risk,
  };
}

export interface ParsedWorkflowToml {
  description: string;
  steps: WorkflowStep[];
  risk: WorkflowRisk;
}

/**
 * Minimal decoder for the `workflow.toml` schema we own. Handles the
 * fields the visual viewer needs (workflow.description, step.name,
 * step.kind, step.description). Inline tables (e.g. `args = {...}`)
 * are ignored; comments are stripped; unknown sections do not abort.
 */
export function parseStepsFromToml(toml: string): ParsedWorkflowToml {
  let workflowDescription = "";
  const stepDrafts: Array<{
    name?: string;
    kind?: string;
    description?: string;
  }> = [];
  let section: "none" | "workflow" | "step" = "none";
  let current: { name?: string; kind?: string; description?: string } | null = null;

  for (const rawLine of toml.split(/\r?\n/)) {
    const stripped = rawLine.replace(/\s+#.*$/, "").replace(/^\s*#.*$/, "").trimEnd();
    const line = stripped.trim();
    if (!line) continue;
    if (line === "[workflow]") {
      if (current) {
        stepDrafts.push(current);
        current = null;
      }
      section = "workflow";
      continue;
    }
    if (line === "[[step]]") {
      if (current) stepDrafts.push(current);
      current = {};
      section = "step";
      continue;
    }
    if (line.startsWith("[")) {
      if (current) {
        stepDrafts.push(current);
        current = null;
      }
      section = "none";
      continue;
    }
    const eq = line.indexOf("=");
    if (eq < 0) continue;
    const key = line.slice(0, eq).trim();
    const rawValue = line.slice(eq + 1).trim();
    const value = stripQuoted(rawValue);
    if (section === "workflow") {
      if (key === "description") workflowDescription = value;
    } else if (section === "step" && current) {
      if (key === "name") current.name = value;
      else if (key === "kind") current.kind = value;
      else if (key === "description") current.description = value;
    }
  }
  if (current) stepDrafts.push(current);

  const steps: WorkflowStep[] = stepDrafts.map((draft, idx) => {
    const kind: WorkflowStepKind = VALID_STEP_KINDS.has(draft.kind as WorkflowStepKind)
      ? (draft.kind as WorkflowStepKind)
      : "tool_call";
    return {
      id: `step-${idx + 1}`,
      kind,
      name: draft.name ?? `step_${idx + 1}`,
      description: draft.description ?? "",
    };
  });
  const risk: WorkflowRisk = steps.some((s) => s.kind === "approval")
    ? "approval-required"
    : "autonomous";
  return { description: workflowDescription, steps, risk };
}

function stripQuoted(value: string): string {
  if (value.length >= 2 && value.startsWith("\"")) {
    const closing = findClosingQuote(value);
    if (closing > 0) {
      return value
        .slice(1, closing)
        .replace(/\\"/g, "\"")
        .replace(/\\\\/g, "\\")
        .replace(/\\n/g, "\n")
        .replace(/\\t/g, "\t");
    }
  }
  if (value.length >= 2 && value.startsWith("'")) {
    const end = value.indexOf("'", 1);
    if (end > 0) return value.slice(1, end);
  }
  return value;
}

function findClosingQuote(value: string): number {
  for (let i = 1; i < value.length; i++) {
    const ch = value[i];
    if (ch === "\\") {
      i += 1;
      continue;
    }
    if (ch === "\"") return i;
  }
  return -1;
}

export type {
  WorkflowRisk,
  WorkflowSpec,
  WorkflowStep,
  WorkflowStepKind,
  WorkflowSummary,
};

// ── Version history + rollback (#1980 gateway routes) ──────────

export interface WorkflowVersionItem {
  id: string;
  version: number;
  parent_id: string | null;
  lifecycle_state: "draft" | "active" | "archived" | string;
  created_at: string;
  modified_by: string;
  definition_toml: string;
}

interface WorkflowVersionListEnvelope {
  vertical_id?: string;
  name?: string;
  count?: number;
  items?: WorkflowVersionItem[];
}

export interface RollbackToVersionArgs {
  verticalId: string;
  name: string;
  targetVersionId: string;
}

export interface RollbackToVersionResult {
  /** ID of the newly created head version. */
  id: string;
  /** Numeric head version after the rollback. */
  version: number;
}

function versionsPath(verticalId: string, name: string): string {
  return `/api/v1/verticals/${encodeURIComponent(verticalId)}/workflows/${encodeURIComponent(name)}/versions`;
}

function rollbackPath(verticalId: string, name: string): string {
  return `/api/v1/verticals/${encodeURIComponent(verticalId)}/workflows/${encodeURIComponent(name)}/rollback`;
}

export async function listVersions(
  verticalId: string,
  name: string,
): Promise<WorkflowVersionItem[]> {
  const envelope = await apiGet<WorkflowVersionListEnvelope>(
    versionsPath(verticalId, name),
  );
  return envelope.items ?? [];
}

export async function rollbackToVersion(
  args: RollbackToVersionArgs,
): Promise<RollbackToVersionResult> {
  return apiPost<RollbackToVersionResult>(rollbackPath(args.verticalId, args.name), {
    target_version_id: args.targetVersionId,
  });
}
