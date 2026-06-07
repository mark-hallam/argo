/**
 * Audit drill-in API client. Mirrors the `audit.ts` 404→mock-fallback
 * contract; the gateway endpoint `GET /api/v1/audit/<actionId>`
 * arrives via `mission-gateway-audit-detail-endpoint`.
 *
 * In strict-production mode (NODE_ENV=production AND
 * NEXT_PUBLIC_ENV=production) the mock fallback is suppressed — a
 * 404 or network error surfaces to the caller so SREs see real
 * gateway breakage rather than mocks that pretend to work.
 *
 * The auth-bypass dev shortcut (NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1)
 * short-circuits to the mock, matching the rest of the audit feed.
 */
import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { findSampleAuditDetail } from "@/lib/sample-audit-detail";
import { hasRealSession, isDemoLive, isDemoMode } from "@/lib/skycot/demo-mode";
import type { AuditDetail } from "@/lib/types/audit-detail";
import type { AuditStatus, DataClassification } from "@/lib/types/audit";

/**
 * Canonical audit-API endpoint paths. The SPA API client builds every
 * audit request from this single source of truth so the contract is
 * pinned in one place (and by `web/tests/surface_contract_audit_paths.test.ts`).
 *
 * | Method | Path                              | Purpose                       |
 * | ------ | --------------------------------- | ----------------------------- |
 * | GET    | `/api/v1/audit`                   | Paginated audit feed          |
 * | GET    | `/api/v1/audit/{actionId}`        | Single-action detail drill-in |
 * | POST   | `/api/v1/audit/{actionId}/replay` | Deterministic action replay   |
 *
 * Deprecated — these older paths were never canonical and MUST NOT be
 * reintroduced anywhere in the API client. They are pinned absent by the
 * surface-contract test:
 *
 * | Deprecated path         | Replaced by                |
 * | ----------------------- | -------------------------- |
 * | `/api/v1/audit/actions` | `/api/v1/audit` (the feed) |
 * | `/api/v1/audit/events`  | `/api/v1/audit` (the feed) |
 */
export const AUDIT_ENDPOINT_PATHS = {
  /** Paginated audit feed. */
  feed: "/api/v1/audit",
  /** Single-action detail drill-in. */
  detail: (actionId: string): string =>
    `/api/v1/audit/${encodeURIComponent(actionId)}`,
  /** Deterministic replay of a single action. */
  replay: (actionId: string): string =>
    `/api/v1/audit/${encodeURIComponent(actionId)}/replay`,
} as const;

/**
 * Deprecated audit paths that must never reappear in the API client.
 * Pinned by `web/tests/surface_contract_audit_paths.test.ts`.
 */
export const DEPRECATED_AUDIT_ENDPOINT_PATHS = [
  "/api/v1/audit/actions",
  "/api/v1/audit/events",
] as const;

export type DelegationRole = "originator" | "delegated_to" | "executed";

export interface DelegationStep {
  workerId: string;
  workerName: string;
  actionId: string;
  role: DelegationRole;
  ts: string;
}

export type AuditDetailWithDelegation = AuditDetail & {
  delegationChain?: DelegationStep[];
};

type JsonObject = Record<string, unknown>;

interface GatewayDetailEnvelope {
  detail?: unknown;
}

export class AuditDetailNotFoundError extends Error {
  actionId: string;
  constructor(actionId: string) {
    super(`Audit action not found: ${actionId}`);
    this.name = "AuditDetailNotFoundError";
    this.actionId = actionId;
  }
}

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  // Per-user gate (sprint, 2026-06-07, mock-leak class): a REAL
  // signed-in user (the `skycot-session=1` marker cookie is present) is
  // NEVER served the auth-bypass mock fixtures, even in a demo-flagged
  // deploy. The bypass short-circuit is for ANONYMOUS demo visitors only.
  if (hasRealSession()) return false;
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

function mockDetail(actionId: string): AuditDetailWithDelegation {
  const detail = findSampleAuditDetail(actionId);
  if (!detail) throw new AuditDetailNotFoundError(actionId);
  return withDelegation(detail);
}

/**
 * Fetch a single audit-action detail. Throws `AuditDetailNotFoundError`
 * when the action id is unknown — both in strict-production (real
 * gateway 404) and in mock/demo modes. The drill-in page maps that
 * typed error onto the `<AuditActionNotFound>` in-shell 404 surface
 * rather than fabricating a synthetic provenance card.
 *
 * Outside strict-production:
 *  - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` short-circuits to mocks,
 *  - `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` (demo) short-circuits to mocks,
 *  - gateway 404 / network errors fall back to mocks so the page
 *    never renders the global 500 boundary for the seeded ids.
 */
export async function fetchAuditDetail(actionId: string): Promise<AuditDetailWithDelegation> {
  if (!actionId) throw new AuditDetailNotFoundError(actionId);

  // `isDemoLive()` is the recording-mode opt-out: with auth-bypass +
  // demo-mode + demo-live, the drill-in hits the real gateway so the
  // audit row the camera lands on is the cryptographically-anchored
  // server row, not a hand-shaped sample.
  if (isAuthBypassed() && !isDemoLive()) {
    return mockDetail(actionId);
  }

  if (isDemoMode() && !isStrictProduction() && !isDemoLive()) {
    return mockDetail(actionId);
  }

  const path = AUDIT_ENDPOINT_PATHS.detail(actionId);
  try {
    return normalizeAuditDetail(await apiGet<unknown>(path));
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      throw new AuditDetailNotFoundError(actionId);
    }
    if (!hasRealSession() && !isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockDetail(actionId);
    }
    throw err;
  }
}

function normalizeAuditDetail(raw: unknown): AuditDetailWithDelegation {
  const value = unwrapDetail(raw);
  if (!isRecord(value)) {
    throw new ApiError(502, "audit-detail endpoint returned an unexpected shape");
  }
  if (isRecord(value.summary)) {
    const summary = value.summary;
    return {
      id: stringValue(summary.id),
      timestamp: stringValue(summary.timestamp),
      worker: normalizeWorker(summary.worker),
      actionType: stringValue(summary.actionType ?? summary.action_type),
      verb: stringValue(summary.verb),
      status: normalizeStatus(summary.status),
      approver: isRecord(summary.approver) ? normalizePerson(summary.approver) : undefined,
      decider_role: stringValue(summary.decider_role) as AuditDetail["decider_role"],
      channel: stringValue(summary.channel) as AuditDetail["channel"],
      cost: isRecord(summary.cost) ? { usd: numberValue(summary.cost.usd) } : undefined,
      dataClassification: normalizeClassification(
        summary.dataClassification ?? summary.data_classification,
      ),
      inputs: normalizeInputs(value.inputs),
      decisionChain: normalizeDecisionChain(value.decisionChain ?? value.decision_chain),
      provenance: normalizeProvenance(value.provenance),
      output: normalizeOutput(value.output),
      delegationChain: normalizeDelegation(value.delegationChain ?? value.delegation_chain),
    };
  }
  return withDelegation(
    value as unknown as AuditDetail & {
      delegationChain?: unknown;
      delegation_chain?: unknown;
    },
  );
}

function unwrapDetail(raw: unknown): unknown {
  if (isRecord(raw) && "detail" in (raw as GatewayDetailEnvelope)) {
    return raw.detail;
  }
  return raw;
}

function withDelegation(
  detail: AuditDetail & { delegationChain?: unknown; delegation_chain?: unknown },
): AuditDetailWithDelegation {
  return {
    ...detail,
    delegationChain: normalizeDelegation(
      detail.delegationChain ?? detail.delegation_chain,
    ),
  };
}

function normalizeInputs(value: unknown): AuditDetail["inputs"] {
  const input = isRecord(value) ? value : {};
  const predecessor = isRecord(input.predecessor)
    ? {
        id: stringValue(input.predecessor.id),
        verb: stringValue(input.predecessor.verb),
      }
    : undefined;
  return {
    triggerKind: stringValue(input.triggerKind ?? input.trigger_kind, "message") as AuditDetail["inputs"]["triggerKind"],
    triggerSummary: stringValue(input.triggerSummary ?? input.trigger_summary),
    predecessor,
  };
}

function normalizeDecisionChain(value: unknown): AuditDetail["decisionChain"] {
  return Array.isArray(value)
    ? value.filter(isRecord).map((step) => ({
        n: numberValue(step.n),
        title: stringValue(step.title),
        body: stringValue(step.body),
        details: isRecord(step.details) ? step.details : undefined,
      }))
    : [];
}

function normalizeProvenance(value: unknown): AuditDetail["provenance"] {
  const p = isRecord(value) ? value : {};
  // Carry the hash-chain anchor through. Every audit row has a
  // `self_hash`, so the gateway always emits `hash_chain_anchor`; the
  // SPA's `ProvenanceBlock` renders it as a "Chain anchor" row, which
  // keeps the card non-empty for meta-event rows (e.g. a "Run now"
  // `worker.run` trigger) that have no model/skill/policy provenance.
  const chainAnchor =
    stringValue(p.hashChainAnchor) || stringValue(p.hash_chain_anchor) || undefined;
  return {
    model: normalizeIdVersion(p.model),
    promptTemplate: normalizeIdHash(p.promptTemplate ?? p.prompt_template),
    skill: normalizeIdVersion(p.skill),
    memorySnapshot: { id: stringValue(asRecord(p.memorySnapshot ?? p.memory_snapshot).id) },
    policy: normalizePolicy(p.policy),
    ...(chainAnchor ? { hash_chain_anchor: chainAnchor } : {}),
  };
}

function normalizeOutput(value: unknown): AuditDetail["output"] {
  const output = isRecord(value) ? value : {};
  return {
    kind: stringValue(output.kind, "text") as AuditDetail["output"]["kind"],
    body: (output.body ?? "") as AuditDetail["output"]["body"],
    screenshots: Array.isArray(output.screenshots)
      ? output.screenshots.filter(isRecord).map((shot) => ({
          url: stringValue(shot.url),
          timestamp: stringValue(shot.timestamp),
          description: stringValue(shot.description),
        }))
      : undefined,
    delivery: isRecord(output.delivery)
      ? {
          sentAt: stringValue(output.delivery.sentAt ?? output.delivery.sent_at),
          receivedAt: stringValue(output.delivery.receivedAt ?? output.delivery.received_at),
          downstreamId: stringValue(output.delivery.downstreamId ?? output.delivery.downstream_id),
        }
      : undefined,
    followUp: isRecord(output.followUp ?? output.follow_up)
      ? {
          id: stringValue(asRecord(output.followUp ?? output.follow_up).id),
          verb: stringValue(asRecord(output.followUp ?? output.follow_up).verb),
        }
      : undefined,
  };
}

function normalizeDelegation(value: unknown): DelegationStep[] {
  if (!Array.isArray(value)) return [];
  return value.filter(isRecord).map((step) => ({
    workerId: stringValue(step.workerId ?? step.worker_id),
    workerName: stringValue(step.workerName ?? step.worker_name, "unknown"),
    actionId: stringValue(step.actionId ?? step.action_id),
    role: normalizeDelegationRole(step.role),
    ts: stringValue(step.ts ?? step.timestamp),
  }));
}

function normalizeDelegationRole(value: unknown): DelegationRole {
  return value === "originator" || value === "delegated_to" || value === "executed"
    ? value
    : "delegated_to";
}

function normalizeWorker(value: unknown): AuditDetail["worker"] {
  const worker = isRecord(value) ? value : {};
  return {
    id: stringValue(worker.id),
    name: stringValue(worker.name, "Unknown worker"),
    avatarUrl: stringValue(worker.avatarUrl ?? worker.avatar_url) || undefined,
  };
}

function normalizePerson(value: JsonObject): { id: string; name: string } {
  return { id: stringValue(value.id), name: stringValue(value.name) };
}

function normalizeIdVersion(value: unknown): { id: string; version: string } {
  const obj = asRecord(value);
  return { id: stringValue(obj.id), version: stringValue(obj.version) };
}

function normalizeIdHash(value: unknown): { id: string; hash: string } {
  const obj = asRecord(value);
  return { id: stringValue(obj.id), hash: stringValue(obj.hash) };
}

function normalizePolicy(value: unknown): AuditDetail["provenance"]["policy"] {
  const obj = asRecord(value);
  return {
    id: stringValue(obj.id),
    version: stringValue(obj.version),
    hash: stringValue(obj.hash),
  };
}

function normalizeStatus(value: unknown): AuditStatus {
  return ["succeeded", "refused", "pending", "failed", "autonomous"].includes(
    String(value),
  )
    ? (value as AuditStatus)
    : "failed";
}

function normalizeClassification(value: unknown): DataClassification | undefined {
  return ["public", "internal", "confidential", "restricted"].includes(String(value))
    ? (value as DataClassification)
    : undefined;
}

function isRecord(value: unknown): value is JsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asRecord(value: unknown): JsonObject {
  return isRecord(value) ? value : {};
}

function stringValue(value: unknown, fallback = ""): string {
  return typeof value === "string" ? value : fallback;
}

function numberValue(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}
