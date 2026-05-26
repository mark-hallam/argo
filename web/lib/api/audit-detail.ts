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
import {
  findSampleAuditDetail,
  placeholderAuditDetail,
} from "@/lib/sample-audit-detail";
import { isDemoMode } from "@/lib/skycot/demo-mode";
import type { AuditDetail } from "@/lib/types/audit-detail";
import type { AuditStatus, DataClassification } from "@/lib/types/audit";

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

/** Demo-mode best-effort lookup. Prefers a hand-shaped or synthesised
 *  sample detail; falls back to a placeholder so the drill-in page
 *  renders rather than 5xx-ing when live-gateway ids don't match the
 *  seed roster. */
function demoModeDetail(actionId: string): AuditDetail {
  return findSampleAuditDetail(actionId) ?? placeholderAuditDetail(actionId);
}

/**
 * Fetch a single audit-action detail. Throws `AuditDetailNotFoundError`
 * when the action id is unknown in strict-production (real gateway
 * 404). Outside strict-production:
 *  - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` short-circuits to mocks,
 *  - `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` (demo) prefers mocks and falls
 *    back to a synthetic placeholder if the action id is unknown,
 *  - gateway 404 / network errors fall back to mock-or-placeholder so
 *    the page never renders the global 500 boundary.
 */
export async function fetchAuditDetail(actionId: string): Promise<AuditDetailWithDelegation> {
  if (!actionId) throw new AuditDetailNotFoundError(actionId);

  if (isAuthBypassed()) {
    return mockDetail(actionId);
  }

  if (isDemoMode() && !isStrictProduction()) {
    return demoModeDetail(actionId);
  }

  const path = `/api/v1/audit/${encodeURIComponent(actionId)}`;
  try {
    return normalizeAuditDetail(await apiGet<unknown>(path));
  } catch (err) {
    if (err instanceof ApiError && err.status === 404 && isStrictProduction()) {
      throw new AuditDetailNotFoundError(actionId);
    }
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockDetail(actionId);
    }
    if (!isStrictProduction()) {
      // Any other dev-mode gateway failure (5xx, parse error, etc.)
      // degrades to a placeholder so Scene 6 doesn't hard-500.
      return demoModeDetail(actionId);
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
  return {
    model: normalizeIdVersion(p.model),
    promptTemplate: normalizeIdHash(p.promptTemplate ?? p.prompt_template),
    skill: normalizeIdVersion(p.skill),
    memorySnapshot: { id: stringValue(asRecord(p.memorySnapshot ?? p.memory_snapshot).id) },
    policy: normalizePolicy(p.policy),
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
