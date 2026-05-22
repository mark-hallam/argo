/**
 * Workflow-version rollback diff helper.
 *
 * Companion to the rollback CLI in
 * `crates/argo-cli/src/handlers/workflow_def.rs` (#1967) and the
 * `WorkflowRolledBack` audit kind (#1980). When an audit row of kind
 * `workflow_rolled_back` is opened at `/audit/action/<id>`, this
 * module extracts the typed rollback payload from the action's JSON
 * output body, runs the same LCS line-diff the CLI uses, and counts
 * step-block changes for the hero's 1-line summary.
 *
 * The Rust LCS lives in `workflow_def_diff.rs`; the equivalent JS
 * port is `compute_unified()` in `./unified.ts` — we re-use it
 * verbatim so the UI's "Show diff" panel byte-matches the CLI's
 * unified diff (modulo unified-diff context windowing).
 *
 * Pure / RSC-safe. No DOM, no Node APIs, no fetch.
 */
import { compute_unified, type DiffHunk, type UnifiedDiff } from "@/lib/diff/unified";
import type { DeciderRole } from "@/lib/types/audit";
import type { AuditDetail } from "@/lib/types/audit-detail";

/**
 * Typed shape of the JSON payload carried in
 * `AuditDetail.output.body` for a `workflow_rolled_back` row. Gateway
 * projection layer assembles this from the persisted audit row
 * (`rolled_back_from_version_id`, `new_version_id`,
 * `new_version_number`) joined against the two `workflow_definition`
 * rows so the renderer has the TOML bodies inline.
 *
 * All fields are optional so the UI can degrade gracefully when a
 * pre-projection row predates a given field (notably `decider_role`,
 * which lands via the pending tick-26 reauthor).
 */
export interface WorkflowRollbackPayload {
  vertical_id?: string;
  workflow_name?: string;
  /** Head version BEFORE rollback (e.g., 5 in "v5 → v3"). */
  from_version?: number;
  /** Target version rolled back TO (e.g., 3 in "v5 → v3"). */
  to_version?: number;
  /** TOML body of the previous head version (left side of diff). */
  before_toml?: string;
  /** TOML body of the rolled-back-to version (right side of diff). */
  after_toml?: string;
  /** Resolved RBAC role of the decider — pairs with the tick-26
   *  reauthored `web-approvals-decider-role-on-timeline` brief. */
  decider_role?: DeciderRole;
  decider_name?: string;
}

/** Counts used for the "12 steps changed, 3 added, 5 modified, 4 removed"
 *  summary line. `total` equals `added + modified + removed`. */
export interface RollbackStepSummary {
  added: number;
  modified: number;
  removed: number;
  total: number;
}

/** Extracts the typed rollback payload from an audit-detail row when
 *  it is a `workflow_rolled_back` action with a JSON output body of
 *  the expected shape. Returns `null` otherwise — the renderer falls
 *  back to the generic hero in that case. */
export function parseRollbackPayload(
  detail: Pick<AuditDetail, "actionType" | "output">,
): WorkflowRollbackPayload | null {
  if (detail.actionType !== "workflow_rolled_back") return null;
  const body = detail.output?.body;
  if (!body || typeof body !== "object") return null;
  const raw = body as Record<string, unknown>;

  const out: WorkflowRollbackPayload = {};
  if (typeof raw.vertical_id === "string") out.vertical_id = raw.vertical_id;
  if (typeof raw.workflow_name === "string") out.workflow_name = raw.workflow_name;
  if (typeof raw.from_version === "number") out.from_version = raw.from_version;
  if (typeof raw.to_version === "number") out.to_version = raw.to_version;
  if (typeof raw.before_toml === "string") out.before_toml = raw.before_toml;
  if (typeof raw.after_toml === "string") out.after_toml = raw.after_toml;
  if (isDeciderRole(raw.decider_role)) out.decider_role = raw.decider_role;
  if (typeof raw.decider_name === "string") out.decider_name = raw.decider_name;
  return out;
}

function isDeciderRole(value: unknown): value is DeciderRole {
  return value === "owner" || value === "admin" || value === "user" || value === "guest";
}

/**
 * Count step-block changes between two workflow-definition TOML
 * bodies. Identity for matching a "step" across versions is the
 * `name = "..."` line inside a `[[steps]]` block — the canonical
 * step identifier in the workflow schema.
 *
 *   - **added**: step name present in `after` but not `before`.
 *   - **removed**: step name present in `before` but not `after`.
 *   - **modified**: step name present on both sides with different
 *     block bodies (any field change inside the step counts).
 *
 * Returns zero-counts when either side is empty.
 */
export function summarizeStepChanges(
  before: string | undefined,
  after: string | undefined,
): RollbackStepSummary {
  const left = parseStepBlocks(before ?? "");
  const right = parseStepBlocks(after ?? "");
  let added = 0;
  let removed = 0;
  let modified = 0;
  for (const [name, body] of left) {
    const rhs = right.get(name);
    if (rhs === undefined) removed++;
    else if (rhs !== body) modified++;
  }
  for (const [name] of right) {
    if (!left.has(name)) added++;
  }
  return { added, modified, removed, total: added + modified + removed };
}

/** Compute the unified line-diff used by the expandable "Show diff"
 *  panel. Returns the same `UnifiedDiff` shape as the existing
 *  `compute_unified` so the renderer reuses any future hunk styling
 *  improvements. */
export function computeRollbackLineDiff(
  payload: Pick<WorkflowRollbackPayload, "before_toml" | "after_toml">,
): UnifiedDiff {
  return compute_unified(payload.before_toml ?? "", payload.after_toml ?? "");
}

/** Re-exported from `./unified` so the rollback-diff component never
 *  has to import two diff modules. */
export type { DiffHunk, UnifiedDiff };

const STEP_BLOCK_RE = /\[\[steps\]\]([\s\S]*?)(?=\n\[\[steps\]\]|\n\[[^\[]|\n*$)/g;
const STEP_NAME_RE = /^\s*name\s*=\s*"([^"]+)"\s*$/m;

/** Build a `name → block-body` map from a workflow TOML body. */
function parseStepBlocks(toml: string): Map<string, string> {
  const out = new Map<string, string>();
  if (!toml) return out;
  for (const match of toml.matchAll(STEP_BLOCK_RE)) {
    const body = match[1] ?? "";
    const nameMatch = body.match(STEP_NAME_RE);
    if (!nameMatch) continue;
    out.set(nameMatch[1], body.trim());
  }
  return out;
}
