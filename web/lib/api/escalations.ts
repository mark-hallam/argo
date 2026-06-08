/**
 * Worker escalation-policy API client.
 *
 * Backs the worker-detail **Escalation** tab. The gateway endpoints
 * (`worker-detail-escalation-policy`) are:
 *   - `GET   /api/v1/workers/<id>/escalations`
 *   - `PATCH /api/v1/workers/<id>/escalations`
 * persisting per-worker rules to `.skycot/worker-escalations/<id>.toml`
 * (the sibling of the worker-policies endpoint).
 *
 * Fallback contract mirrors `lib/api/worker-overview.ts`:
 *  - anonymous auth-bypass (no real session, NOT demo-live) → sample
 *    rules immediately, no gateway call.
 *  - a REAL signed-in user, or demo-live, calls the live endpoint.
 *  - 404 / network error in non-strict dev → sample fallback.
 *  - strict production propagates all errors.
 */

import { apiGet, apiPatch } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import type { EscalationRule } from "@/components/skycot/workers/EscalationMatrix";
import { hasRealSession, isDemoLive } from "@/lib/skycot/demo-mode";
import { isMissingOrUnreachable } from "./unreachable";

export type EscalationSource = "gateway" | "mock-offline" | "mock-bypass";

export interface WorkerEscalationPolicy {
  rules: EscalationRule[];
  /** True when the worker has no operator-authored rules persisted. */
  isDefault: boolean;
  /** True when the worker is paused (HalfOpen): rules are read-only. */
  readOnly: boolean;
}

export interface FetchWorkerEscalationsResult {
  policy: WorkerEscalationPolicy;
  source: EscalationSource;
}

/** Raw rule shape as the live gateway returns it (snake_case). */
interface RawGatewayRule {
  action?: string;
  condition?: string;
  routes_to?: string;
  channel?: string | null;
}

interface RawGatewayEscalations {
  rules?: RawGatewayRule[];
  is_default?: boolean;
  read_only?: boolean;
}

/**
 * A generic, worker-agnostic sample policy used as the offline /
 * anonymous-demo fallback. Deliberately small: the live endpoint is
 * the source of truth once a real session or demo-live is in play.
 */
const SAMPLE_RULES: EscalationRule[] = [
  {
    action: "Reply to a customer message",
    condition: "Confidence in the next action is below 80%",
    routesTo: "On-call operator",
    channel: "Slack #skycot-escalations",
  },
  {
    action: "Trigger a public or irreversible side-effect",
    condition: "Action would post publicly or move money",
    routesTo: "Worker owner",
    channel: "Email",
  },
];

function mapGatewayRule(raw: RawGatewayRule): EscalationRule {
  const channel = raw.channel ?? undefined;
  return {
    action: raw.action ?? "",
    condition: raw.condition ?? "",
    routesTo: raw.routes_to ?? "",
    channel: channel && channel.trim() !== "" ? channel : undefined,
  };
}

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  // A REAL signed-in user is never served the bypass fixtures, even in
  // a demo-flagged deploy. The bypass short-circuit is for ANONYMOUS
  // demo visitors only (mirrors worker-overview.ts).
  if (hasRealSession()) return false;
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function sampleResult(source: EscalationSource): FetchWorkerEscalationsResult {
  return {
    policy: { rules: SAMPLE_RULES, isDefault: true, readOnly: false },
    source,
  };
}

/** Map a live gateway payload onto the SPA policy contract. */
function mapPolicy(raw: RawGatewayEscalations): WorkerEscalationPolicy {
  return {
    rules: (raw.rules ?? []).map(mapGatewayRule),
    isDefault: raw.is_default ?? (raw.rules ?? []).length === 0,
    readOnly: raw.read_only ?? false,
  };
}

export async function fetchWorkerEscalations(
  workerId: string,
): Promise<FetchWorkerEscalationsResult> {
  // Anonymous auth-bypass mocks the gateway; demo-live opts back into
  // a real round-trip so the recording-day SPA proves the live path.
  if (isAuthBypassed() && !isDemoLive()) {
    return sampleResult("mock-bypass");
  }

  try {
    const raw = await apiGet<RawGatewayEscalations>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/escalations`,
    );
    return { policy: mapPolicy(raw), source: "gateway" };
  } catch (err) {
    // Fall back to the sample only when anonymous/offline in non-strict
    // mode. A real session or demo-live surfaces the error instead of
    // silently fabricating rules.
    if (
      !hasRealSession() &&
      !isDemoLive() &&
      !isStrictProduction() &&
      isMissingOrUnreachable(err)
    ) {
      return sampleResult("mock-offline");
    }
    if (err instanceof ApiError && err.status === 404) {
      return sampleResult("mock-offline");
    }
    throw err;
  }
}

export interface UpdateWorkerEscalationsInput {
  rules: EscalationRule[];
  operatorNote: string;
}

/**
 * Persist a worker's escalation rule set via the live PATCH endpoint.
 * The wire body uses snake_case (`routes_to`, `operator_note`) to match
 * the gateway contract.
 */
export async function updateWorkerEscalations(
  workerId: string,
  input: UpdateWorkerEscalationsInput,
): Promise<WorkerEscalationPolicy> {
  const body = {
    rules: input.rules.map((r) => ({
      action: r.action,
      condition: r.condition,
      routes_to: r.routesTo,
      ...(r.channel ? { channel: r.channel } : {}),
    })),
    operator_note: input.operatorNote,
  };
  const raw = await apiPatch<RawGatewayEscalations>(
    `/api/v1/workers/${encodeURIComponent(workerId)}/escalations`,
    body,
  );
  return mapPolicy(raw);
}
