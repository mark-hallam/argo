"use client";

import { useMutation } from "@tanstack/react-query";
import { apiGet, apiPost, ApiError } from "@/lib/api/client";
import { findWorkerByName } from "@/lib/api/team";
import { appendDemoWorker } from "@/lib/api/demo-roster-store";
import type {
  GenesisVerticalReadyEvent,
  GenesisGateDetail,
  RoleContract,
} from "@/lib/api/genesis";
import type { WorkerOverview } from "@/lib/api/worker-overview";
import { uuidV7 } from "@/lib/hire/idempotency-key";
import { isDemoMode } from "@/lib/skycot/demo-mode";
import type { Worker } from "@/lib/types/worker";

const VERIFY_ATTEMPTS = 3;
const VERIFY_BACKOFF_MS = 500;

export const FIRST_HIRE_VERIFY_FAILED_MESSAGE =
  "Hiring succeeded — your teammate is provisioning. Refresh in a few seconds; if they don't appear, the Help link in the top right has next steps.";

/** Surfaced when a confirm re-hold (digest mismatch / re-gate) happens — the
 *  contract changed underfoot, so the operator must regenerate from scratch. */
export const FIRST_HIRE_CONFIRM_RE_HELD_MESSAGE =
  "Couldn't confirm this teammate — please start over and regenerate.";

export interface FirstHireRequest {
  name: string;
  description: string;
  verticalId: string;
  tenantId?: string | null;
}

export interface FirstHireResult {
  workerId: string;
  workerName: string;
  /**
   * `true` when the gateway accepted the request but Genesis will
   * finish hiring asynchronously. The wizard still redirects to the
   * profile page; the profile renders a "hiring…" placeholder until
   * the live worker_id is ready.
   */
  pending: boolean;
}

/**
 * The `needs_review` payload the gateway returns (HTTP 422
 * `genesis_gate_failed`) when the first-hire POST omits the confirmed
 * contract digest. Carries the TRUTHFUL role contract Genesis derived plus
 * the exact `contractDigest` string the operator must echo back at the
 * confirm call. NEVER computed/forged client-side — it is read verbatim off
 * the caught `ApiError.body` (DEC-025 #1, human-review-before-activation).
 */
export interface FirstHireReview {
  name: string;
  description: string;
  verticalId: string;
  tenantId?: string | null;
  roleContract: RoleContract;
  /** The gateway-returned SHA-256 digest. Echoed verbatim at confirm. */
  contractDigest: string;
  gate?: GenesisGateDetail;
}

/**
 * Discriminated outcome of {@link firstHire}. `needs_review` is RETURNED
 * (not thrown) so the page can render the review→confirm screen; a hard
 * block (or any other failure) still throws so the error banner surfaces it.
 */
export type FirstHireOutcome =
  | { kind: "hired"; workerId: string; workerName: string; pending: boolean }
  | { kind: "needs_review"; review: FirstHireReview };

interface FirstHirePayload {
  name: string;
  description: string;
  vertical_id: string;
  /**
   * The operator's confirmation of the reviewed role contract. Present
   * ONLY on the confirm call — and ONLY ever set to the gateway-returned
   * `contract_digest` string (never a client-derived value). The gateway
   * re-derives the digest from the bound manifest and activates on a match.
   */
  confirmed_contract_digest?: string;
}

interface FirstHireResponse {
  worker_id: string;
  worker_name?: string;
  pending?: boolean;
}

interface FirstHireVerifyFailedAuditPayload {
  action: "wizard.first_hire_verify_failed";
  metadata: {
    worker_id: string;
    tenant_id: string | null;
    vertical_id: string;
  };
  tenant: string;
}

function isFirstHireResponse(value: unknown): value is FirstHireResponse {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return typeof v.worker_id === "string" && v.worker_id.length > 0;
}

function waitForRetry(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function emitFirstHireVerifyFailed(
  workerId: string,
  tenantId: string | null | undefined,
  verticalId: string,
): Promise<void> {
  const payload: FirstHireVerifyFailedAuditPayload = {
    action: "wizard.first_hire_verify_failed",
    metadata: {
      worker_id: workerId,
      tenant_id: tenantId ?? null,
      vertical_id: verticalId,
    },
    tenant: tenantId ?? "unknown",
  };
  try {
    await apiPost<void, FirstHireVerifyFailedAuditPayload>("/api/v1/audit", payload);
  } catch (err) {
    if (typeof console !== "undefined") {
      console.warn("first-hire verification audit emission failed", err);
    }
  }
}

async function verifyWorkerReadable(workerId: string): Promise<void> {
  for (let attempt = 1; attempt <= VERIFY_ATTEMPTS; attempt += 1) {
    try {
      await apiGet<unknown>(
        `/api/v1/workers/${encodeURIComponent(workerId)}/overview`,
      );
      return;
    } catch (err) {
      const retryableNotFound = err instanceof ApiError && err.status === 404;
      if (!retryableNotFound || attempt === VERIFY_ATTEMPTS) {
        throw err;
      }
      await waitForRetry(VERIFY_BACKOFF_MS);
    }
  }
}

/**
 * Vertical ids the pick-vertical demo grid filters against. F10
 * (#2647) restricted the grid to verticals with a matching sample
 * persona so the camera never landed on a "Sales SDR → Mariah's
 * marketplace-lister charter" vocabulary mismatch. The
 * VERTICAL_TO_SAMPLE_WORKER map that originally derived this set is
 * gone (the demo short-circuit now synthesises a fresh worker from
 * the user's typed name + description), so the set is now a flat
 * literal. Loosening the F10 filter is out of scope for this brief.
 */
export const DEMO_SUPPORTED_VERTICAL_IDS: ReadonlySet<string> = new Set([
  "marketplace-seller",
  "recruiter",
  "customer-support",
  "marketing-intern",
  "project-coordination",
]);

function buildDemoFirstHireOverview(description: string): WorkerOverview {
  // Empty-baseline KPIs: a brand-new hire has no track record yet.
  // Borrowing another worker's numbers would be theater the camera
  // sees on /workers/<id>/overview.
  return {
    charter: description,
    duties: [],
    kpis: [
      { id: "k-actions", label: "actions taken today", value: "0",
        series: [0, 0, 0, 0, 0, 0, 0], deltaPct: 0, trend: "flat", tone: "muted" },
      { id: "k-approvals", label: "approvals awaiting you", value: "0",
        series: [0, 0, 0, 0, 0, 0, 0], deltaPct: 0, trend: "flat", tone: "muted" },
      { id: "k-shadow", label: "shadow-mode hours", value: "0h",
        series: [0, 0, 0, 0, 0, 0, 0], deltaPct: 0, trend: "flat", tone: "muted" },
      { id: "k-onboarding", label: "onboarding progress", value: "Just hired",
        series: [0, 0, 0, 0, 0, 0, 0], deltaPct: 0, trend: "flat", tone: "muted" },
    ],
    recentActivity: [],
  };
}

function buildDemoFirstHireWorker(
  workerId: string,
  name: string,
  description: string,
  verticalId: string,
): Worker {
  return {
    id: workerId,
    name,
    role: { tagline: description, vertical: verticalId },
    status: "idle",
    lastActiveAt: new Date().toISOString(),
  };
}

function buildDemoFirstHireResult(req: FirstHireRequest): FirstHireResult {
  // Per brief `fix-demo-first-hire-discards-prompt-and-redirects-to-
  // sample-worker`: the prior `VERTICAL_TO_SAMPLE_WORKER` map silently
  // discarded the user's typed name + description and routed every
  // vertical to a fixed Mariah/Priya/Jamal/Nori/Peta. The camera saw
  // the wrong name on /workers/<id> and the roster never grew.
  // Synthesize a real-shape UUIDv7 + append the hire to the demo
  // roster so /team + /workers + /workers/<id>/overview all reflect
  // the user's actual input.
  const workerId = uuidV7();
  appendDemoWorker({
    worker: buildDemoFirstHireWorker(workerId, req.name, req.description, req.verticalId),
    overview: buildDemoFirstHireOverview(req.description),
  });
  return { workerId, workerName: req.name, pending: false };
}

/**
 * The gateway's HTTP 422 `genesis_gate_failed` envelope, as parsed onto
 * `ApiError.body`. Every field is optional so a forward/backward-compat
 * gateway can't crash the type-guard; the guard below enforces what the
 * review path actually requires.
 */
interface GateBlockedBody {
  error?: { code?: string; message?: string };
  role_contract?: RoleContract | null;
  contract_digest?: string;
  gate?: GenesisGateDetail;
}

function asGateBlockedBody(body: unknown): GateBlockedBody | null {
  if (!body || typeof body !== "object") return null;
  return body as GateBlockedBody;
}

/**
 * A 422 is a confirmable `needs_review` ONLY when the gate explicitly says
 * `needs_review` AND the gateway handed us BOTH a non-empty role contract
 * (at least one action) and a non-empty digest to echo back. Anything else —
 * a `blocked` gate, a `genesis_plan_failed` code, a missing contract/digest —
 * is a HARD block and must be thrown, never rendered as confirmable.
 */
function isConfirmableNeedsReview(
  err: ApiError,
): err is ApiError & { body: Required<Pick<GateBlockedBody, "role_contract" | "contract_digest">> & GateBlockedBody } {
  if (err.status !== 422) return false;
  if (err.code !== "genesis_gate_failed") return false;
  const body = asGateBlockedBody(err.body);
  if (!body) return false;
  if (body.gate?.state !== "needs_review") return false;
  const contract = body.role_contract;
  if (!contract || !Array.isArray(contract.actions) || contract.actions.length === 0) {
    return false;
  }
  return typeof body.contract_digest === "string" && body.contract_digest.length > 0;
}

/**
 * Build the `FirstHireReview` payload from a confirmable 422 body. The digest
 * is copied VERBATIM from the gateway response — never hashed, derived, or
 * recomputed client-side (SAFETY GUARDRAIL #1).
 */
function reviewFromGateBody(req: FirstHireRequest, body: GateBlockedBody): FirstHireReview {
  return {
    name: req.name,
    description: req.description,
    verticalId: req.verticalId,
    tenantId: req.tenantId,
    roleContract: body.role_contract as RoleContract,
    contractDigest: body.contract_digest as string,
    gate: body.gate,
  };
}

/**
 * The honest hard-block message to throw for a NON-confirmable 422 (gate
 * `blocked`, `genesis_plan_failed`, or a 422 missing the contract/digest).
 * Prefers the gateway's own `error.message`, falling back to the ApiError
 * message. The throw keeps the wizard on-step so the operator can retry.
 */
function hardBlockError(err: ApiError): ApiError {
  const body = asGateBlockedBody(err.body);
  const message = body?.error?.message || err.message;
  return new ApiError(err.status, message, err.code, err.body);
}

/**
 * Shared post-201 path for BOTH the initial `firstHire` (the legacy
 * pre-digest 201, if the gateway still hands one back) and the
 * `confirmFirstHire` confirm call: read back the freshly-hired worker, fall
 * back to a by-name lookup on the ghost-worker 404, and surface
 * `FIRST_HIRE_VERIFY_FAILED_MESSAGE` if it stays unreadable.
 */
async function settleHiredResponse(
  raw: FirstHireResponse,
  req: FirstHireRequest,
): Promise<{ kind: "hired"; workerId: string; workerName: string; pending: boolean }> {
  try {
    await verifyWorkerReadable(raw.worker_id);
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      // Defence-in-depth: a 404 here historically meant a gateway "ghost
      // worker" (write/read tenant mismatch). The handler-side fix lives
      // in `routes_onboarding.rs::first_hire_response`, but if the SPA
      // still hits 404 (e.g. a future regression of that same class), try
      // resolving the freshly hired teammate by name against the team
      // roster before surfacing `FIRST_HIRE_VERIFY_FAILED_MESSAGE`.
      const fallbackName = raw.worker_name;
      if (fallbackName) {
        const hit = await findWorkerByName(fallbackName);
        if (hit) {
          return {
            kind: "hired",
            workerId: hit.id,
            workerName: hit.name,
            pending: raw.pending ?? false,
          };
        }
      }
      await emitFirstHireVerifyFailed(raw.worker_id, req.tenantId, req.verticalId);
      throw new ApiError(404, FIRST_HIRE_VERIFY_FAILED_MESSAGE, "first_hire_verify_failed");
    }
    throw err;
  }
  return {
    kind: "hired",
    workerId: raw.worker_id,
    workerName: raw.worker_name ?? "your first worker",
    pending: raw.pending ?? false,
  };
}

export async function firstHire(req: FirstHireRequest): Promise<FirstHireOutcome> {
  // Demo-mode short-circuit: the first-hire endpoint requires a verified
  // principal AND a real tenant id. Neither exists in demo mode. Skip
  // the gateway + the post-hire readback verify and synthesize a
  // session-local hire keyed off the user's typed name + description;
  // `appendDemoWorker` makes the new worker resolvable from /team and
  // /workers/<id>/overview without a live gateway.
  if (isDemoMode()) {
    return { kind: "hired", ...buildDemoFirstHireResult(req) };
  }
  let raw: unknown;
  try {
    raw = await apiPost<unknown, FirstHirePayload>(
      "/api/v1/onboarding/first-hire",
      { name: req.name, description: req.description, vertical_id: req.verticalId },
    );
  } catch (err) {
    // DEC-025 #1: the gateway runs Genesis, derives the truthful capability
    // contract, and holds the hire in `needs_review` — HTTP 422 — because no
    // `confirmed_contract_digest` was sent. That is NOT a failure: it is the
    // human-review-before-activation step. RETURN it so the page can render
    // the review→confirm screen. A hard `blocked` gate (or any other error)
    // still throws.
    if (err instanceof ApiError && isConfirmableNeedsReview(err)) {
      return { kind: "needs_review", review: reviewFromGateBody(req, err.body) };
    }
    if (err instanceof ApiError && err.status === 422) {
      // 422 that is NOT a confirmable review (blocked / plan-failed / missing
      // contract or digest) — surface the gateway's honest block message.
      throw hardBlockError(err);
    }
    throw err;
  }
  if (!isFirstHireResponse(raw)) {
    throw new ApiError(502, "first-hire endpoint returned an unexpected shape");
  }
  // Legacy/no-gate path: the gateway returned a 201 worker directly (e.g. the
  // gate is OFF for this tenant). Settle it exactly as before.
  return settleHiredResponse(raw, req);
}

/**
 * Confirm a reviewed hire. Re-POSTs `/api/v1/onboarding/first-hire` with the
 * SAME inputs PLUS `confirmed_contract_digest` set to the EXACT digest the
 * gateway returned in the 422 review body (SAFETY GUARDRAIL #1 — never a
 * client-derived value). May ONLY be called after the operator has reviewed
 * and confirmed via `RoleContractReview` (SAFETY GUARDRAIL #2). On a 201 the
 * worker is read back and returned as `kind:"hired"`; a second 422 (digest
 * mismatch / re-hold) throws a "start over" error.
 */
export async function confirmFirstHire(review: FirstHireReview): Promise<FirstHireOutcome> {
  const req: FirstHireRequest = {
    name: review.name,
    description: review.description,
    verticalId: review.verticalId,
    tenantId: review.tenantId,
  };
  let raw: unknown;
  try {
    raw = await apiPost<unknown, FirstHirePayload>("/api/v1/onboarding/first-hire", {
      name: review.name,
      description: review.description,
      vertical_id: review.verticalId,
      confirmed_contract_digest: review.contractDigest,
    });
  } catch (err) {
    if (err instanceof ApiError && err.status === 422) {
      // The digest no longer matches the bound contract (it changed underfoot)
      // or the gate re-held — confirming again won't help; the operator must
      // regenerate from scratch.
      throw new ApiError(
        422,
        FIRST_HIRE_CONFIRM_RE_HELD_MESSAGE,
        err.code ?? "genesis_gate_failed",
        err.body,
      );
    }
    throw err;
  }
  if (!isFirstHireResponse(raw)) {
    throw new ApiError(502, "first-hire endpoint returned an unexpected shape");
  }
  return settleHiredResponse(raw, req);
}

/**
 * Adapter: project a `FirstHireReview` into the minimal
 * `GenesisVerticalReadyEvent` shape `RoleContractReview` consumes, so the
 * onboarding review screen REUSES the exact `/genesis` review component.
 *
 * `judge_verdict` is mapped from the gate state so `needs_review` →
 * "Awaiting your review" and `blocked` → the non-hireable blocked note.
 * `sop_uploaded` is deliberately left UNSET so the SOP badge never renders
 * for the text-only onboarding path. Every role-contract array field is
 * defensively defaulted to `[]` so a sparse gateway payload can't crash the
 * component.
 */
export function gateBodyToVerticalReady(
  review: FirstHireReview,
): GenesisVerticalReadyEvent {
  const rc = review.roleContract;
  const safeContract: RoleContract = {
    role_name: rc.role_name,
    does: rc.does,
    actions: Array.isArray(rc.actions) ? rc.actions : [],
    approval_required: Array.isArray(rc.approval_required) ? rc.approval_required : [],
    autonomous: Array.isArray(rc.autonomous) ? rc.autonomous : [],
    escalation: rc.escalation,
    will_not: Array.isArray(rc.will_not) ? rc.will_not : [],
    draft_integrations: Array.isArray(rc.draft_integrations) ? rc.draft_integrations : [],
    cadence: rc.cadence,
    exceptions: rc.exceptions,
    success_criteria: rc.success_criteria,
  };
  return {
    vertical_id: review.verticalId,
    vertical_dir_path: "",
    tenant_id: review.tenantId ?? "",
    manifest: {
      id: review.verticalId,
      name: safeContract.role_name,
      archetype: "",
      persona: "",
      actions_declared: safeContract.actions.length,
    },
    prompts: [],
    skills: [],
    eval_seed_count: review.gate?.eval_total ?? 0,
    judge_verdict: review.gate?.state ?? "needs_review",
    role_contract: safeContract,
    contract_digest: review.contractDigest,
    gate: review.gate,
  };
}

export function useFirstHire() {
  return useMutation<FirstHireOutcome, Error, FirstHireRequest>({
    mutationFn: firstHire,
  });
}

export function useConfirmFirstHire() {
  return useMutation<FirstHireOutcome, Error, FirstHireReview>({
    mutationFn: confirmFirstHire,
  });
}
