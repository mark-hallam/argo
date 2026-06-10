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
import { hasRealSession, isDemoLive } from "@/lib/skycot/demo-mode";
import { publishHead, saveDraft } from "@/lib/api/workflows-save-publish";
import { encodeStepsToToml } from "@/app/(app)/workflows/[verticalId]/[name]/build/step-encoder";
import {
  SAMPLE_WORKFLOW_SPECS,
  SAMPLE_WORKFLOW_SUMMARIES,
  classifyAuthor,
  type WorkflowAuthorType,
  type WorkflowRisk,
  type WorkflowSpec,
  type WorkflowStep,
  type WorkflowStepKind,
  type WorkflowSummary,
} from "@/lib/sample-workflows";

export type WorkflowDataSource = "gateway" | "mock-offline" | "mock-bypass";
export type WorkflowSubstrate = "macos" | "linux" | "windows" | "e2b" | "browser" | "headless";
export type WorkflowSummaryWithSubstrate = WorkflowSummary & { substrate?: WorkflowSubstrate };
export type WorkflowSpecWithSubstrate = WorkflowSpec & { substrate?: WorkflowSubstrate };

export interface FetchAllWorkflowsResult {
  workflows: WorkflowSummaryWithSubstrate[];
  source: WorkflowDataSource;
}

export interface FetchWorkflowResult {
  workflow: WorkflowSpecWithSubstrate | null;
  source: WorkflowDataSource;
}

interface WorkflowListEnvelope {
  vertical_id?: string;
  count?: number;
  items?: GatewayWorkflowItem[];
}

/// Response shape for the top-level `/api/v1/workflows` route.
/// Used in recording-mode (`isDemoLive()`) so a single round-trip
/// surfaces every tenant workflow across all verticals, including
/// ones not in the local sample list (e.g. `marketplace`).
interface TopLevelWorkflowListEnvelope {
  tenant_id?: string;
  total_count?: number;
  workflows?: GatewayWorkflowItem[];
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
  /** Server-side discriminant (#fix-workflow-card-author-type-discriminant-chip). Falls through to client `classifyAuthor` when absent. */
  author_type?: string;
  author_worker_id?: string | null;
  definition_toml?: string;
  substrate?: string | null;
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
  // Recording-mode opt-out (`NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`):
  // route the catalog through the top-level `/api/v1/workflows`
  // endpoint so the demo MP4's narration ("these are the workflows
  // my teammates run") is literally true — and verticals like
  // `marketplace` not in the local sample fan-out list still
  // surface in one round-trip.
  // A REAL signed-in user AND demo-live both read the top-level
  // `/api/v1/workflows` endpoint. Two reasons a real user MUST take this
  // path: (1) the per-vertical `/verticals/*` fan-out below is CORS-broken
  // for signed-in browser calls (see the module header), and (2) a real
  // user must NEVER fall back to `SAMPLE_WORKFLOW_SUMMARIES` — those are
  // demo personas, and showing them in a real workspace was the
  // stranger-signup defect. Demo-live keeps the seeded SAMPLE fallback so
  // the recording stays camera-safe; a real user falls back to an EMPTY
  // catalog (their genuine "no workflows yet" state) instead.
  const realSession = hasRealSession();
  if (isDemoLive() || realSession) {
    try {
      const list = await apiGet<TopLevelWorkflowListEnvelope>("/api/v1/workflows");
      const items = list.workflows ?? [];
      if (items.length > 0) {
        return {
          workflows: items.map(summaryFromGatewayItem),
          source: "gateway",
        };
      }
      return realSession
        ? { workflows: [], source: "gateway" }
        : { workflows: SAMPLE_WORKFLOW_SUMMARIES, source: "mock-offline" };
    } catch (err) {
      if (!isStrictProduction() && isRecoverable(err)) {
        return realSession
          ? { workflows: [], source: "gateway" }
          : { workflows: SAMPLE_WORKFLOW_SUMMARIES, source: "mock-offline" };
      }
      throw err;
    }
  }
  // Anonymous auth-bypass dev/demo (no real session) shows the seeded deck.
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
  // Recording-mode opt-out (`NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`): skip the
  // mock short-circuit so the Scene-5 workflow-detail surface
  // (`/workflows/<vertical>/<name>`) fires a real
  // `GET /api/v1/verticals/<vertical>/workflows/<name>` round-trip
  // instead of serving the bundled seed spec. Matches the gate the
  // `fetchAllWorkflows` list already uses above and the canonical
  // `isAuthBypassed() && !isDemoLive()` pattern in the other api modules.
  if (isAuthBypassed() && !isDemoLive()) {
    const local = SAMPLE_WORKFLOW_SPECS[key];
    if (local) {
      return { workflow: local, source: "mock-bypass" };
    }
    // No local sample for this key — fall through to the network so
    // page.route stubs in tests (and a live gateway in dev) can serve
    // a workflow that the seed doesn't carry.
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

export function summaryFromGatewayItem(item: GatewayWorkflowItem): WorkflowSummaryWithSubstrate {
  const parsed = parseStepsFromToml(item.definition_toml ?? "");
  const description = parsed.description || `Workflow ${item.name} in ${item.vertical_id}.`;
  const lastEditor = item.modified_by ?? "—";
  const author = resolveAuthor(item, lastEditor);
  return {
    name: item.name,
    vertical: item.vertical_id,
    description,
    steps: parsed.steps.length,
    versions: Math.max(1, Number(item.version) || 1),
    lastEdited: item.created_at ?? "—",
    lastEditor,
    authorType: author.authorType,
    authorWorkerId: author.authorWorkerId,
    risk: parsed.risk,
    substrate: normalizeWorkflowSubstrate(item.substrate) ?? parsed.substrate,
  };
}

function resolveAuthor(
  item: GatewayWorkflowItem,
  lastEditor: string,
): { authorType: WorkflowAuthorType; authorWorkerId?: string } {
  const declared = item.author_type;
  if (declared === "worker" || declared === "human" || declared === "template_pack") {
    return {
      authorType: declared,
      authorWorkerId: item.author_worker_id ?? undefined,
    };
  }
  return classifyAuthor(lastEditor);
}

export function specFromGatewayItem(item: GatewayWorkflowItem): WorkflowSpecWithSubstrate {
  const parsed = parseStepsFromToml(item.definition_toml ?? "");
  return {
    name: item.name,
    vertical: item.vertical_id,
    description: parsed.description || `Workflow ${item.name} in ${item.vertical_id}.`,
    version: `v${Math.max(1, Number(item.version) || 1)}`,
    steps: parsed.steps,
    risk: parsed.risk,
    substrate: normalizeWorkflowSubstrate(item.substrate) ?? parsed.substrate,
  };
}

export interface ParsedWorkflowToml {
  description: string;
  steps: WorkflowStep[];
  risk: WorkflowRisk;
  substrate: WorkflowSubstrate;
}

/**
 * Minimal decoder for the `workflow.toml` schema we own. Handles the
 * fields the visual viewer needs (workflow.description,
 * workflow.default_substrate, step.name/kind/description/tool).
 * Inline tables are ignored; comments are stripped; unknown sections
 * do not abort.
 */
export function parseStepsFromToml(toml: string): ParsedWorkflowToml {
  let workflowDescription = "";
  const stepDrafts: Array<{
    name?: string;
    kind?: string;
    description?: string;
    tool?: string;
  }> = [];
  let explicitSubstrate: WorkflowSubstrate | null = null;
  let section: "none" | "workflow" | "step" = "none";
  let current: { name?: string; kind?: string; description?: string; tool?: string } | null =
    null;

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
      else if (key === "substrate" || key === "default_substrate") {
        explicitSubstrate = normalizeWorkflowSubstrate(value) ?? explicitSubstrate;
      }
    } else if (section === "step" && current) {
      if (key === "name") current.name = value;
      else if (key === "kind") current.kind = value;
      else if (key === "description") current.description = value;
      else if (key === "tool") current.tool = value;
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
  return {
    description: workflowDescription,
    steps,
    risk,
    substrate: explicitSubstrate ?? inferSubstrateFromDrafts(stepDrafts),
  };
}

export function normalizeWorkflowSubstrate(value: unknown): WorkflowSubstrate | null {
  if (typeof value !== "string") return null;
  const normalized = value.trim().toLowerCase().replace(/[_\s]+/g, "-");
  if (["macos", "mac", "darwin", "computer-use-macos", "cu-macos"].includes(normalized)) {
    return "macos";
  }
  if (normalized === "linux") return "linux";
  if (normalized === "windows" || normalized === "win32") return "windows";
  if (normalized === "e2b" || normalized === "sandbox") return "e2b";
  if (normalized === "browser" || normalized === "web") return "browser";
  if (normalized === "headless" || normalized === "none") return "headless";
  return null;
}

export function resolveWorkflowSubstrate(
  workflow:
    | { substrate?: unknown; name?: string; description?: string; steps?: unknown }
    | null
    | undefined,
): WorkflowSubstrate {
  if (!workflow) return "headless";
  const explicit = normalizeWorkflowSubstrate(workflow.substrate);
  if (explicit) return explicit;
  const steps = Array.isArray(workflow.steps) ? workflow.steps : [];
  const joined = [
    workflow.name,
    workflow.description,
    ...steps.flatMap((step) => {
      if (!step || typeof step !== "object") return [];
      const s = step as { kind?: string; name?: string; description?: string; tool?: string };
      return [s.kind, s.name, s.description, s.tool];
    }),
  ].join(" ");
  return inferSubstrateFromText(joined);
}

export function stepUsesSubstrate(
  step: { kind?: string; name?: string; description?: string },
  substrate: WorkflowSubstrate,
): boolean {
  if (substrate === "headless") return false;
  const text = `${step.kind ?? ""} ${step.name ?? ""} ${step.description ?? ""}`;
  if (inferSubstrateFromText(text) === substrate) return true;
  return ["tool_call", "http_request", "code"].includes(step.kind ?? "");
}

function inferSubstrateFromDrafts(
  drafts: Array<{ kind?: string; name?: string; description?: string; tool?: string }>,
): WorkflowSubstrate {
  return inferSubstrateFromText(
    drafts.flatMap((draft) => [draft.kind, draft.name, draft.description, draft.tool]).join(" "),
  );
}

function inferSubstrateFromText(text: string): WorkflowSubstrate {
  const lower = text.toLowerCase();
  if (
    lower.includes("cu_action") ||
    lower.includes("cu.") ||
    lower.includes("cu_") ||
    lower.includes("computer-use") ||
    lower.includes("computer use") ||
    lower.includes("macos") ||
    lower.includes("macos_notes") ||
    lower.includes("notes app")
  ) {
    return "macos";
  }
  if (lower.includes("windows")) return "windows";
  if (lower.includes("linux")) return "linux";
  if (lower.includes("e2b") || lower.includes("sandbox")) return "e2b";
  if (lower.includes("browser") || lower.includes("web page")) return "browser";
  return "headless";
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

// ── Seed publish-then-run bridge ───────────────────────────────

/**
 * Materialize a seed-source workflow as a real gateway row, then promote
 * it to head, so a subsequent execute runs against actual persisted state
 * rather than the bundled sample. The detail page's Run-now action calls
 * this when `source !== "gateway"`: the gateway has no row for this
 * workflow yet, so we serialize the seed steps to `workflow.toml`, POST
 * them as a draft, and PUT that draft as head. Returns the canonical
 * numeric head version the gateway recorded.
 *
 * This keeps the demo run honest (NO THEATER — DEC-007/DEC-029): the run
 * the camera sees fires the two backends against a freshly-published row
 * threaded with the operator's principal, never a mock dispatch. When a
 * concurrent actor has already published the workflow the gateway reports
 * a conflict and the live head is run as-is.
 *
 * @example
 * const headVersion = await publishSeedWorkflowToHead("support", "approve_refund", spec);
 */
export async function publishSeedWorkflowToHead(
  verticalId: string,
  name: string,
  workflow: Pick<WorkflowSpec, "steps" | "description">,
): Promise<number> {
  const definition_toml = encodeStepsToToml(workflow.steps, {
    workflowName: name,
    workflowDescription: workflow.description,
  });
  const saved = await saveDraft({
    vertical_id: verticalId,
    name,
    definition_toml,
  });
  if (saved.kind === "conflict") {
    // Head already advanced (another actor published this workflow in the
    // meantime) — the live head IS the row we want to run, so skip the
    // promote and report its version.
    return saved.current_version;
  }
  const { head_version } = await publishHead({
    vertical_id: verticalId,
    name,
    version_id: saved.version_id,
  });
  return head_version;
}
