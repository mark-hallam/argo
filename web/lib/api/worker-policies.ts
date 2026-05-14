/**
 * Per-worker approval-policy API client
 * (mission-approval-policy-ui). Mirrors `worker-settings.ts`'s
 * mock-fallback contract (`NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` →
 * mock immediately; 404/network in dev → mock; strict production
 * propagates).
 */

import { apiGet, apiPatch } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

export type ApprovalPolicy =
  | "auto-approve"
  | "always-ask"
  | "always-refuse"
  | "risk-gated";

export const APPROVAL_POLICIES: readonly ApprovalPolicy[] = [
  "auto-approve",
  "always-ask",
  "always-refuse",
  "risk-gated",
] as const;

export const APPROVAL_POLICY_LABELS: Record<ApprovalPolicy, string> = {
  "auto-approve": "Auto-approve",
  "always-ask": "Always ask",
  "always-refuse": "Always refuse",
  "risk-gated": "Risk-gated (default)",
};

export const APPROVAL_POLICY_HELP: Record<ApprovalPolicy, string> = {
  "auto-approve": "Skip the approval queue. Use for reversible actions only.",
  "always-ask": "Always queue for human approval regardless of risk.",
  "always-refuse": "Block dispatch entirely until policy changes.",
  "risk-gated": "Use the manifest's risk class — low auto-approves, high asks.",
};

export type InferredTrustGrade = "low" | "medium" | "high" | "unknown";

export interface WorkerPolicyRow {
  actionType: string;
  currentPolicy: ApprovalPolicy;
  inferredTrustGrade: InferredTrustGrade;
  isDefault: boolean;
}

export type WorkerPoliciesSource =
  | "gateway"
  | "mock-offline"
  | "mock-bypass";

export interface WorkerPoliciesResult {
  workerId: string;
  readOnly: boolean;
  rows: WorkerPolicyRow[];
  source: WorkerPoliciesSource;
}

interface PoliciesWireRow {
  action_type: string;
  current_policy: string;
  inferred_trust_grade: string;
  is_default: boolean;
}

interface PoliciesWireResponse {
  worker_id: string;
  read_only: boolean;
  policies: PoliciesWireRow[];
  allowed_policies: string[];
  default_policy: string;
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
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

function asPolicy(s: string): ApprovalPolicy {
  if ((APPROVAL_POLICIES as readonly string[]).includes(s)) {
    return s as ApprovalPolicy;
  }
  return "risk-gated";
}

function asGrade(s: string): InferredTrustGrade {
  if (s === "low" || s === "medium" || s === "high") return s;
  return "unknown";
}

function fromWire(
  workerId: string,
  raw: PoliciesWireResponse,
  source: WorkerPoliciesSource,
): WorkerPoliciesResult {
  return {
    workerId,
    readOnly: raw.read_only,
    rows: raw.policies.map((r) => ({
      actionType: r.action_type,
      currentPolicy: asPolicy(r.current_policy),
      inferredTrustGrade: asGrade(r.inferred_trust_grade),
      isDefault: r.is_default,
    })),
    source,
  };
}

function mockResult(
  workerId: string,
  source: WorkerPoliciesSource,
): WorkerPoliciesResult {
  return {
    workerId,
    readOnly: false,
    source,
    rows: [
      { actionType: "draft_listing", currentPolicy: "auto-approve", inferredTrustGrade: "low", isDefault: false },
      { actionType: "post_listing", currentPolicy: "risk-gated", inferredTrustGrade: "medium", isDefault: true },
      { actionType: "reply_to_buyer", currentPolicy: "always-ask", inferredTrustGrade: "medium", isDefault: false },
      { actionType: "mark_item_sold", currentPolicy: "risk-gated", inferredTrustGrade: "high", isDefault: true },
    ],
  };
}

export async function getWorkerPolicies(
  workerId: string,
): Promise<WorkerPoliciesResult> {
  if (isAuthBypassed()) return mockResult(workerId, "mock-bypass");
  try {
    const raw = await apiGet<PoliciesWireResponse>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/policies`,
    );
    return fromWire(workerId, raw, "gateway");
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockResult(workerId, "mock-offline");
    }
    throw err;
  }
}

export interface UpdatePolicyInput {
  workerId: string;
  actionType: string;
  policy: ApprovalPolicy;
  operatorNote: string;
}

export interface UpdatePolicyResult {
  workerId: string;
  actionType: string;
  policy: ApprovalPolicy;
  previousPolicy: ApprovalPolicy;
  inferredTrustGrade: InferredTrustGrade;
  source: WorkerPoliciesSource;
}

interface UpdateWireResponse {
  worker_id: string;
  action_type: string;
  policy: string;
  previous_policy: string;
  operator_note: string;
  inferred_trust_grade: string;
}

export async function updateWorkerPolicy(
  input: UpdatePolicyInput,
): Promise<UpdatePolicyResult> {
  const url = `/api/v1/workers/${encodeURIComponent(input.workerId)}/policies/${encodeURIComponent(
    input.actionType,
  )}`;
  const payload = { policy: input.policy, operator_note: input.operatorNote };

  if (isAuthBypassed()) {
    return {
      workerId: input.workerId,
      actionType: input.actionType,
      policy: input.policy,
      previousPolicy: "risk-gated",
      inferredTrustGrade: "unknown",
      source: "mock-bypass",
    };
  }
  try {
    const raw = await apiPatch<UpdateWireResponse>(url, payload);
    return {
      workerId: raw.worker_id,
      actionType: raw.action_type,
      policy: asPolicy(raw.policy),
      previousPolicy: asPolicy(raw.previous_policy),
      inferredTrustGrade: asGrade(raw.inferred_trust_grade),
      source: "gateway",
    };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return {
        workerId: input.workerId,
        actionType: input.actionType,
        policy: input.policy,
        previousPolicy: "risk-gated",
        inferredTrustGrade: "unknown",
        source: "mock-offline",
      };
    }
    throw err;
  }
}
