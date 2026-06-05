/**
 * Genesis-lite classification API client.
 *
 * Calls `POST /api/v1/genesis/classify` via the typed gateway client.
 * v1 falls back to the local `STUB_QUESTIONS` set on 404 / malformed
 * response so the hire flow keeps moving when the gateway-side
 * endpoint isn't wired yet (sibling brief
 * `mission-gateway-genesis-classify-endpoint`). 5xx and network
 * errors are re-thrown so the upstream TanStack Query retry loop can
 * apply backoff before falling back to the stub.
 *
 * Acceptance contract:
 * - Happy path → returns `{ questions, source: "gateway" }`.
 * - 404 / malformed → returns `{ questions: STUB_QUESTIONS, source: "stub" }`.
 * - 5xx / network → throws (caller retries; eventually falls back to stub).
 */

import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { resolveGatewayUrl } from "@/lib/config/gateway-url";
import { STUB_QUESTIONS, stubWorkerDraft } from "@/lib/hire/genesis-stub";
import {
  fromGatewayWorkerDraft,
  type GatewayDraftResponse,
  type GatewayWorkerDraft,
  type WorkerDraft,
} from "@/lib/hire/draft-types";
import type {
  AnswerType,
  ClassifyResponse,
  Question,
  QuestionAnswer,
} from "@/lib/hire/genesis-types";

export type GenesisSource = "gateway" | "stub";

export interface ClassifyResult {
  questions: Question[];
  source: GenesisSource;
  classification?: string;
}

export interface ClassifyRequest {
  problem: string;
}

const ANSWER_TYPES: ReadonlySet<AnswerType> = new Set<AnswerType>([
  "single-select",
  "multi-select",
  "free-text",
  "yes-no",
]);

function isQuestion(value: unknown): value is Question {
  if (!value || typeof value !== "object") return false;
  const q = value as Record<string, unknown>;
  if (typeof q.id !== "string" || q.id.length === 0) return false;
  if (typeof q.text !== "string" || q.text.length === 0) return false;
  if (typeof q.answer_type !== "string") return false;
  if (!ANSWER_TYPES.has(q.answer_type as AnswerType)) return false;
  if (q.answer_type === "single-select" || q.answer_type === "multi-select") {
    if (!Array.isArray(q.options) || q.options.length === 0) return false;
    if (!q.options.every((opt) => typeof opt === "string")) return false;
  }
  return true;
}

function isClassifyResponse(value: unknown): value is ClassifyResponse {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  if (!Array.isArray(v.questions)) return false;
  if (v.questions.length === 0 || v.questions.length > 3) return false;
  return v.questions.every(isQuestion);
}

function stubResult(): ClassifyResult {
  return { questions: [...STUB_QUESTIONS], source: "stub" };
}

/**
 * Classify a Step 1 problem statement into clarifying questions.
 *
 * Returns the gateway's response on success, or a stub fallback when
 * the gateway returns 404 or a malformed payload. Throws on 5xx /
 * network so the upstream retry-with-backoff loop can re-attempt
 * before flipping to the stub.
 */
export async function classifyRole(req: ClassifyRequest): Promise<ClassifyResult> {
  let raw: unknown;
  try {
    raw = await apiPost<unknown>("/api/v1/genesis/classify", { problem: req.problem });
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      return stubResult();
    }
    throw err;
  }

  if (!isClassifyResponse(raw)) {
    return stubResult();
  }

  return {
    questions: raw.questions,
    source: "gateway",
    classification: raw.classification,
  };
}

export interface DraftWorkerRequest {
  problem: string;
  answers: Record<string, QuestionAnswer>;
  /**
   * Free-text re-draft hint surfaced by Step 4's Adjust drawer
   * (E2-e). Server-side Genesis treats this as additional context
   * ("make her more formal", "shorter replies", etc.). v1 stub
   * ignores it — the user still gets the generic draft until the
   * gateway endpoint lands. Optional so Step 3's first-pass draft
   * call stays unchanged.
   */
  adjustment?: string;
}

export interface DraftWorkerResult {
  draft: WorkerDraft;
  source: GenesisSource;
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((v) => typeof v === "string");
}

/**
 * Type-guard for the canonical wire shape returned by
 * `POST /api/v1/genesis/draft-worker` — the flat string-only struct
 * defined at `crates/argo-cli/src/handlers/gateway/routes_genesis.rs`.
 * The richer nested TS [[WorkerDraft]] is reconstructed from this by
 * [[fromGatewayWorkerDraft]] at the boundary. Prior to the
 * `web-hire-workerdraft-schema-mirror-only` brief, this validator
 * mistakenly expected the rich shape and silently fell back to the
 * STUB draft on every gateway hit — root cause of the "5 hires →
 * 5 Mariah clones" bug.
 */
function isGatewayWorkerDraft(value: unknown): value is GatewayWorkerDraft {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  if (typeof v.suggestedName !== "string") return false;
  if (typeof v.persona !== "string") return false;
  if (typeof v.charter !== "string") return false;
  if (!isStringArray(v.duties)) return false;
  if (!isStringArray(v.exampleReplies)) return false;
  if (typeof v.approvalPolicy !== "string") return false;
  return typeof v.suggestedVertical === "string";
}

function isGatewayDraftResponse(value: unknown): value is GatewayDraftResponse {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  if (!isGatewayWorkerDraft(v.worker_draft)) return false;
  return v.cost_usd === null || typeof v.cost_usd === "number";
}

/**
 * Generate a worker draft from a Step 1 problem statement + Step 2
 * answers. Returns the gateway's draft on success, or the stub
 * fallback when the gateway returns 404 / malformed payload. Throws
 * on 5xx / network so the caller can apply a retry strategy.
 */
export async function draftWorker(req: DraftWorkerRequest): Promise<DraftWorkerResult> {
  let raw: unknown;
  const payload: Record<string, unknown> = {
    problem: req.problem,
    answers: req.answers,
  };
  if (req.adjustment !== undefined && req.adjustment.length > 0) {
    payload.adjustment = req.adjustment;
  }
  try {
    raw = await apiPost<unknown>("/api/v1/genesis/draft-worker", payload);
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      return { draft: stubWorkerDraft(req.problem), source: "stub" };
    }
    throw err;
  }

  if (!isGatewayDraftResponse(raw)) {
    return { draft: stubWorkerDraft(req.problem), source: "stub" };
  }

  return {
    draft: fromGatewayWorkerDraft(raw.worker_draft),
    source: "gateway",
  };
}

// ---------------------------------------------------------------------------
// `POST /api/v1/genesis` — streaming SSE client + hire-from-Genesis call.
//
// Per `mission-genesis-spa-page-describe-and-watch`, the /genesis SPA page
// fires the streaming endpoint, renders the live progress strip + preview
// card from the events, then turns `vertical_ready` into a real
// `POST /api/v1/workers` hire so the operator lands on `/workers/{id}`.
// ---------------------------------------------------------------------------

/** Phase ids the Rust streaming handler emits via `event: progress`.
 *  `ingesting_sop` only fires when the operator attached an SOP document
 *  (Genesis P6); it sits between bundle-generation and seed-validation. */
export const GENESIS_PHASES = [
  "drafting_persona",
  "compiling_charter",
  "generating_skill_bundle",
  "ingesting_sop",
  "validating_eval_seeds",
  "running_safety_gate",
  "ready",
] as const;
export type GenesisPhase = (typeof GENESIS_PHASES)[number];

export interface GenesisProgressEvent {
  phase: GenesisPhase;
  message: string;
  percent: number;
}

export interface GenesisVerticalReadyManifest {
  id: string;
  name: string;
  archetype: string;
  persona: string;
  actions_declared: number;
}

/** One action in the teammate's bound vocabulary, projected for the
 *  operator review screen (DEC-025 #4). Mirrors `argo_genesis::ContractAction`. */
export interface RoleContractAction {
  action: string;
  description: string;
  requires_approval: boolean;
  risk: string;
  executor: string;
}

/** One typed, measurable success criterion captured at intake — HOW the
 *  teammate's performance is judged. Reuses the manifest KPI vocabulary
 *  (`kind` + `comparator`). Mirrors `argo_genesis::SuccessCriterion`. */
export interface SuccessCriterion {
  /** Operator-facing metric label (e.g. `"first response time"`). */
  name: string;
  /** Metric type tag — `percentage` | `duration_ms` | `count` | `ratio` |
   *  `score`. Controls unit/format downstream. */
  kind: string;
  /** The pass/fail line the operator chose, interpreted via `comparator`. */
  target: number;
  /** Pass direction — `gte` | `lte` | `eq`. */
  comparator: string;
}

/** The truthful, operator-confirmable role contract derived from the
 *  teammate's ACTUAL bound capability profile (DEC-025 #4). The SPA renders
 *  this as the "what can this teammate do" review screen; the operator
 *  confirms it by echoing `GenesisVerticalReadyEvent.contract_digest` back at
 *  the hire call. Mirrors `argo_genesis::RoleContract`.
 *
 *  `cadence` / `exceptions` / `success_criteria` are intake-context slots
 *  (WHEN it runs, WHAT carve-outs apply, HOW success is measured), enriched
 *  display-only and excluded from the governance digest. Optional for
 *  forward-compat with older gateways. */
export interface RoleContract {
  role_name: string;
  does: string;
  actions: RoleContractAction[];
  approval_required: string[];
  autonomous: string[];
  escalation: string;
  will_not: string[];
  draft_integrations: string[];
  cadence?: string | null;
  exceptions?: string[];
  success_criteria?: SuccessCriterion[];
}

/** The honest gate evidence block the SSE `vertical_ready` payload carries
 *  (mirrors the Rust `gate` object). Fields the SPA reads to render the moat
 *  surface; all optional for forward-compat with older gateways. */
export interface GenesisGateDetail {
  state?: string;
  eval_passed?: number;
  eval_total?: number;
  comprehension_passed?: number;
  comprehension_total?: number;
  /** Genesis P6: whether an SOP was uploaded + graded. */
  sop_applicable?: boolean;
  sop_passed?: number;
  sop_total?: number;
  sop_quiz_questions?: number;
  safety_verdict?: string;
  reasons?: string[];
}

export interface GenesisVerticalReadyEvent {
  vertical_id: string;
  vertical_dir_path: string;
  tenant_id: string;
  manifest: GenesisVerticalReadyManifest;
  prompts: string[];
  skills: string[];
  eval_seed_count: number;
  judge_verdict: string;
  /** The truthful role contract for the operator review screen (DEC-025 #4).
   *  `null` only when no manifest was available to derive one. */
  role_contract?: RoleContract | null;
  /** SHA-256 of the role contract's capability projection. The operator
   *  echoes this back at the hire call to CONFIRM the surfaced contract;
   *  without it the Genesis hire is held in `needs_review`
   *  (human-review-before-activation, DEC-025 #1). */
  contract_digest?: string;
  /** Genesis P6 — the moat: `true` when the operator attached a SOP document
   *  and the teammate was graded against a doc-grounded comprehension quiz. */
  sop_uploaded?: boolean;
  /** The ingested SOP's source label (filename), when one was uploaded. */
  sop_source_label?: string | null;
  /** Honest gate evidence (eval / comprehension / SOP / safety pass-rates). */
  gate?: GenesisGateDetail;
}

export interface GenesisErrorEvent {
  phase: GenesisPhase | string;
  message: string;
  recoverable: boolean;
}

export interface GenesisStreamCallbacks {
  onProgress(ev: GenesisProgressEvent): void;
  onVerticalReady(ev: GenesisVerticalReadyEvent): void;
  onError(ev: GenesisErrorEvent): void;
}

export interface StreamGenesisRequest {
  roleDescription: string;
  /** Optional persona slug override; gateway derives one if omitted. */
  persona?: string;
  /** Display/audit tenant id forwarded as `X-Argo-Tenant-Id`. Under DEC-025 #9
   *  (single-tenant-per-cwd for v1) the gateway no longer routes the generated
   *  vertical's `.genesis/<tenant>/` segment off this header: BOTH the Genesis
   *  SSE write AND the hire-commit read resolve the cwd-derived workspace tenant
   *  (`ensure_workspace`), so the manifest is always written and read in the
   *  same segment regardless of this value. (Previously the SSE wrote under this
   *  header while the hire ignored it and read the cwd tenant → a stale/unique
   *  header produced `422 genesis_gate_failed`.) Still sent for the audit trail
   *  + as the display tenant; multi-tenant header routing is the DEFERRED
   *  enterprise follow-up. */
  tenantId: string;
  /** Wipes any pre-existing `.genesis/<tenant>/<persona>/` before
   *  writing. The first attempt for a role should leave this `false`;
   *  the "Try again" retry sets `true` so the second attempt for the
   *  same role doesn't trip the idempotency check.
   *  See `fix-genesis-idempotency-output-dir-not-empty`. */
  overwrite?: boolean;
  /** Optional single SOP / playbook document the teammate must comprehend
   *  before hire (Genesis P6 — the moat; DEC-025 #5). Plain text, OR a
   *  base64-encoded PDF when `sopDocumentFormat === "pdf_base64"`. When set,
   *  the gateway ingests it into tenant-scoped knowledge and the teammate must
   *  pass a doc-grounded comprehension quiz to clear the hire gate. */
  sopDocument?: string;
  /** How to interpret `sopDocument`: `"text"` (default) or `"pdf_base64"`. */
  sopDocumentFormat?: "text" | "pdf_base64";
  /** Operator-supplied filename for the SOP (surfaced on the review screen). */
  sopFilename?: string;
  /** Aborts the in-flight stream when the caller unmounts / retries. */
  signal?: AbortSignal;
}

function gatewayBaseForSse(): string {
  // SSE bypasses the JSON `request` helper because we need to consume
  // the response body incrementally — but it must resolve its base URL
  // the SAME way `lib/api/client.ts::gatewayBase()` does, or the stream
  // breaks where the equivalent JSON calls succeed.
  //
  // In the browser, return "" so the fetch targets the same-origin
  // `/api/v1/genesis` path served by Next's `/api/v1/:path*` rewrite
  // (`web/next.config.ts`). Same-origin means the browser never makes a
  // cross-origin request, so CORS — and the `TypeError: Failed to fetch`
  // that the absolute `http://127.0.0.1:7400` base produced on the
  // localhost:3000 dev page — never enters the picture. SSR /
  // route-handler contexts run inside the Next process (no `window`) and
  // must use the absolute gateway URL because the Next process, not a
  // browser, is the requester.
  if (typeof window !== "undefined") return "";
  return resolveGatewayUrl();
}

/**
 * Parse a single SSE `event:`/`data:` block produced by the Rust
 * streaming handler. Returns the decoded JSON payload plus the event
 * name, or `null` if the block lacks either field.
 */
export function parseSseBlock(
  block: string,
): { event: string; data: unknown } | null {
  let event = "";
  const dataLines: string[] = [];
  for (const rawLine of block.split("\n")) {
    const line = rawLine.replace(/\r$/, "");
    if (line.startsWith("event:")) {
      event = line.slice("event:".length).trim();
    } else if (line.startsWith("data:")) {
      dataLines.push(line.slice("data:".length).trim());
    }
  }
  if (!event || dataLines.length === 0) return null;
  try {
    return { event, data: JSON.parse(dataLines.join("\n")) };
  } catch {
    return null;
  }
}

/**
 * Dispatch one parsed SSE block to the right callback. Exported so the
 * unit tests can drive the dispatcher synthetically without spinning a
 * real fetch.
 */
export function dispatchGenesisEvent(
  block: string,
  cb: GenesisStreamCallbacks,
): void {
  const parsed = parseSseBlock(block);
  if (!parsed) return;
  switch (parsed.event) {
    case "progress":
      cb.onProgress(parsed.data as GenesisProgressEvent);
      break;
    case "vertical_ready":
      cb.onVerticalReady(parsed.data as GenesisVerticalReadyEvent);
      break;
    case "error":
      cb.onError(parsed.data as GenesisErrorEvent);
      break;
    default:
      // unknown event name — ignore for forward-compat
      break;
  }
}

/**
 * POST `/api/v1/genesis` and dispatch SSE events to `callbacks` as the
 * gateway streams them back. Resolves when the stream closes; rejects
 * on network / non-2xx failure so callers can surface a clear error.
 *
 * The same `tenantId` still flows into the follow-up `hireFromGenesis` call
 * for the audit/display trail, but it is no longer the mechanism that keeps
 * the two endpoints in lockstep: under DEC-025 #9 the gateway resolves the
 * cwd-derived workspace tenant for BOTH the SSE write and the hire read, so
 * the `.genesis/<tenant>/` segment matches by construction regardless of this
 * header. (Multi-tenant header routing is the DEFERRED enterprise follow-up.)
 */
export async function streamGenesis(
  req: StreamGenesisRequest,
  callbacks: GenesisStreamCallbacks,
): Promise<void> {
  const url = `${gatewayBaseForSse()}/api/v1/genesis`;
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Accept: "text/event-stream",
    "X-Argo-Tenant-Id": req.tenantId,
  };
  const body = JSON.stringify({
    role_description: req.roleDescription,
    ...(req.persona !== undefined ? { persona: req.persona } : {}),
    ...(req.overwrite ? { overwrite: true } : {}),
    // Genesis P6 — the moat (DEC-025 #5): optional single SOP doc the teammate
    // must comprehend before hire. Omitted entirely when no SOP is attached so
    // the wire shape is unchanged for the text-only path.
    ...(req.sopDocument
      ? {
          sop_document: req.sopDocument,
          sop_document_format: req.sopDocumentFormat ?? "text",
          ...(req.sopFilename ? { sop_filename: req.sopFilename } : {}),
        }
      : {}),
  });
  const res = await fetch(url, {
    method: "POST",
    headers,
    body,
    signal: req.signal,
    cache: "no-store",
  });
  if (!res.ok || !res.body) {
    const text = await res.text().catch(() => "");
    throw new ApiError(
      res.status,
      `Genesis stream failed: HTTP ${res.status}`,
      undefined,
      text || undefined,
    );
  }
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    let sepIdx: number;
    while ((sepIdx = buffer.indexOf("\n\n")) !== -1) {
      const block = buffer.slice(0, sepIdx);
      buffer = buffer.slice(sepIdx + 2);
      dispatchGenesisEvent(block, callbacks);
    }
  }
  if (buffer.trim().length > 0) {
    dispatchGenesisEvent(buffer, callbacks);
  }
}

export interface HireFromGenesisRequest {
  verticalReady: GenesisVerticalReadyEvent;
  roleDescription: string;
  tenantId: string;
}

export interface HireFromGenesisResult {
  workerId: string;
}

/**
 * Build a canonical `HireDraft` (snake_case → wire camelCase) from the
 * SSE `vertical_ready` payload. The Rust validator requires:
 *
 * - `suggestedName` non-empty
 * - `charter` >= 20 chars
 * - `suggestedVertical` non-empty + resolvable in the catalog (or
 *   `.genesis/<tenant>/`)
 * - at least one duty with `defaultEnabled = true`
 * - `approvalPolicy.level` in {conservative, balanced, aggressive}
 */
function buildHireDraftFromGenesis(
  ready: GenesisVerticalReadyEvent,
  roleDescription: string,
): Record<string, unknown> {
  const name = (ready.manifest.name || "Operator").trim();
  // Charter has to clear the 20-char floor; pad with the role description
  // so we never trip the validator on a terse manifest name.
  const charter = [
    `${name} owns this work end-to-end.`,
    roleDescription.trim(),
  ]
    .join("\n\n")
    .slice(0, 4000);
  // One default duty so the hire validator's "at least one
  // defaultEnabled" requirement passes. The SPA's Phase-2 access-
  // controls panel rewrites duties post-hire; this seed exists only so
  // the worker isn't inert at creation time.
  const duties = [
    {
      cron: "0 9 * * 1-5",
      entryAction: "respond_to_inbound",
      description: "Pick up inbound messages on the morning shift.",
      defaultEnabled: true,
    },
  ];
  return {
    suggestedName: name,
    charter,
    suggestedVertical: ready.vertical_id,
    approvalPolicy: { level: "balanced" },
    duties,
    tagline: ready.manifest.persona || ready.manifest.archetype,
    // DEC-025: carry the Genesis plan id so the persisted worker gets a
    // `genesis_plan_id=` provenance marker. This is load-bearing for
    // SAFETY, not just the provenance chip: the runtime capability fence
    // (P2) AND the enforced probation window (P3) both key off this
    // marker. Without it the flagship SSE-hired teammate would be hired
    // UNFENCED (owner `*` keys) and with NO probation — strictly weaker
    // than the `/onboarding/first-hire` path, which already sets it.
    genesisPlanId: ready.vertical_id,
  };
}

/**
 * POST `/api/v1/workers` with the SSE-derived draft. Surfaces the
 * gateway's `{ workerId }` (or `{ worker_id }`) field as `workerId` so
 * the SPA can route to `/workers/{id}`.
 *
 * Sends the SAME `X-Argo-Tenant-Id` header as the streaming call for a
 * consistent audit/display tenant. Note: under DEC-025 #9 the hire endpoint
 * resolves the cwd-derived workspace tenant (NOT this header) for its vertical
 * lookup, and the SSE writes under that same cwd tenant — so the freshly-
 * written `.genesis/<tenant>/<vertical_id>/` is found in the matching segment
 * by construction, regardless of this header.
 */
export async function hireFromGenesis(
  req: HireFromGenesisRequest,
): Promise<HireFromGenesisResult> {
  const worker_draft = buildHireDraftFromGenesis(
    req.verticalReady,
    req.roleDescription,
  );
  // DEC-025 #1 + #4: confirm the role contract the operator just reviewed in
  // the streamed `vertical_ready` payload by echoing its `contract_digest`.
  // Without this the gateway holds the hire in `needs_review` (human-review-
  // before-activation). The digest binds the confirmation to THIS specific
  // truthful contract — if the teammate's bound capabilities differ from what
  // was surfaced, the gateway re-derives a different digest and refuses.
  const body: Record<string, unknown> = {
    problem: req.roleDescription,
    answers: { source: "genesis_stream" },
    worker_draft,
  };
  if (req.verticalReady.contract_digest) {
    body.confirmed_contract_digest = req.verticalReady.contract_digest;
  }
  const raw = await apiPost<unknown>("/api/v1/workers", body, {
    headers: { "X-Argo-Tenant-Id": req.tenantId },
  });
  if (raw && typeof raw === "object") {
    const obj = raw as { workerId?: unknown; worker_id?: unknown };
    const workerId =
      typeof obj.workerId === "string"
        ? obj.workerId
        : typeof obj.worker_id === "string"
          ? obj.worker_id
          : null;
    if (workerId) {
      return { workerId };
    }
  }
  throw new ApiError(
    502,
    "Hire endpoint returned an unexpected payload",
    "INVALID_HIRE_RESPONSE",
  );
}
