/**
 * Vertical-template marketplace API client.
 *
 * The gateway exposes the loaded vertical-template registry under
 * `/verticals` (`crates/argo-cli/src/handlers/gateway/routes_verticals.rs`).
 * The list response envelope is `{ ok, loaded: VerticalRow[], errors[] }`;
 * the detail endpoint returns the manifest summary plus a `marketplace`
 * envelope with workflows + actions + KPIs.
 *
 * Acceptance contract (mirrors `lib/api/skills.ts`):
 * - 404 from the gateway → fall back to a seed catalog in dev so the
 *   marketplace landing still renders.
 * - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` → mock immediately.
 * - Strict prod (`NODE_ENV === 'production'` AND `NEXT_PUBLIC_ENV ===
 *   'production'`): no mock fallback; ApiError propagates.
 * - NetworkError (gateway unreachable) → mock in dev, propagate in prod.
 */

import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

export type VerticalDataSource = "gateway" | "mock-offline" | "mock-bypass";

export interface VerticalRow {
  id: string;
  name: string;
  version: string;
  description: string;
  /** First sentence of `description`, surfaced as the marketplace card sub-line. */
  headline: string;
  tags: string[];
  /** Number of workflow files declared in the manifest. */
  workflow_count: number;
  /** Number of declared actions in the manifest's `[actions]` block. */
  action_count: number;
  /** Display label for the vertical's role (currently the vertical name). */
  role_label: string;
  /** KPI definition count (declarative + legacy success_metrics). */
  kpi_count: number;
}

interface VerticalsEnvelope {
  ok?: boolean;
  loaded?: VerticalRow[];
  errors?: Array<{ dir: string; error: string }>;
}

export interface FetchVerticalsResult {
  verticals: VerticalRow[];
  source: VerticalDataSource;
}

export interface VerticalActionEntry {
  name: string;
  description: string;
  risk: string;
  requires_approval: boolean;
}

export interface VerticalKpiEntry {
  name: string;
  kind: string;
  threshold: number;
  comparator: string;
  gate: string;
}

export interface VerticalMarketplaceEnvelope {
  headline: string;
  role_label: string;
  workflows: Array<{ file: string }>;
  workflow_count: number;
  actions: VerticalActionEntry[];
  action_count: number;
  kpis: VerticalKpiEntry[];
  tags: string[];
}

export interface VerticalDetail {
  id: string;
  name: string;
  version: string;
  description: string;
  headline?: string;
  tags?: string[];
  authors?: string[];
  role?: { system_prompt?: string; persona?: string | null } | null;
  provider?: { default: string; fallback?: string | null; cost_tier: string } | null;
  tools?: { allowed: string[]; denied: string[] } | null;
  eval_suites?: string[];
  gateway_endpoints?: string[];
  memory_entries?: Array<{ key: string; type: string; purpose?: string }> | null;
  marketplace?: VerticalMarketplaceEnvelope;
}

export interface FetchVerticalResult {
  vertical: VerticalDetail;
  source: VerticalDataSource;
}

/**
 * Seed catalog for dev / offline gateway. Mirrors the verticals that
 * ship with the binary so the operator sees realistic cards even
 * before the gateway is reachable. Kept short — the gateway is the
 * canonical source once it answers.
 */
export const SEED_VERTICALS: VerticalRow[] = [
  {
    id: "finance-ar",
    name: "Finance — Accounts Receivable",
    version: "0.1.0",
    description:
      "Reviews open invoices, drafts polite reminder emails on a 7/14/30-day cadence, escalates to a human at 45 days past due, logs payment receipts, and produces a weekly aging report.",
    headline:
      "Reviews open invoices, drafts polite reminder emails on a 7/14/30-day cadence, escalates to a human at 45 days past due.",
    tags: ["finance", "ar", "gmail", "stripe"],
    workflow_count: 0,
    action_count: 4,
    role_label: "Finance — Accounts Receivable",
    kpi_count: 0,
  },
  {
    id: "gtm-engineer",
    name: "GTM Engineer",
    version: "0.1.0",
    description:
      "Go-to-market automation: lead enrichment, sequencing, signal triage and CRM hygiene with operator approval gates on outbound.",
    headline:
      "Go-to-market automation: lead enrichment, sequencing, signal triage and CRM hygiene with operator approval gates on outbound.",
    tags: ["gtm", "sales", "marketing"],
    workflow_count: 0,
    action_count: 5,
    role_label: "GTM Engineer",
    kpi_count: 0,
  },
  {
    id: "customer-support",
    name: "Customer Support",
    version: "0.1.0",
    description:
      "Tier-1 ticket triage, FAQ replies, escalation routing — every buyer-facing send goes through an approval gate.",
    headline: "Tier-1 ticket triage, FAQ replies, escalation routing.",
    tags: ["support", "tickets"],
    workflow_count: 0,
    action_count: 5,
    role_label: "Customer Support",
    kpi_count: 0,
  },
  {
    id: "marketing-intern",
    name: "Marketing Intern",
    version: "0.1.0",
    description:
      "Content drafting, social posts, weekly competitive scans, brand-voice consistency checks — every public-facing artifact is approval-gated.",
    headline: "Content drafting, social posts, weekly competitive scans.",
    tags: ["marketing", "content"],
    workflow_count: 0,
    action_count: 5,
    role_label: "Marketing Intern",
    kpi_count: 0,
  },
  {
    id: "sales-sdr",
    name: "Sales SDR",
    version: "0.1.0",
    description:
      "Outbound prospecting and meeting booking with approval gates on every external send.",
    headline: "Outbound prospecting and meeting booking with approval gates.",
    tags: ["sales", "outbound"],
    workflow_count: 0,
    action_count: 5,
    role_label: "Sales SDR",
    kpi_count: 0,
  },
];

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

/** GET `/api/v1/verticals` — list every vertical-template the gateway has loaded. */
export async function fetchVerticals(): Promise<FetchVerticalsResult> {
  if (isAuthBypassed()) {
    return { verticals: SEED_VERTICALS, source: "mock-bypass" };
  }
  try {
    const envelope = await apiGet<VerticalsEnvelope>("/api/v1/verticals");
    const verticals = envelope.loaded ?? [];
    if (verticals.length === 0 && !isStrictProduction()) {
      return { verticals: SEED_VERTICALS, source: "mock-offline" };
    }
    return { verticals, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { verticals: SEED_VERTICALS, source: "mock-offline" };
    }
    throw err;
  }
}

function seedDetail(id: string): VerticalDetail {
  const row = SEED_VERTICALS.find((v) => v.id === id) ?? SEED_VERTICALS[0];
  return {
    id: row.id,
    name: row.name,
    version: row.version,
    description: row.description,
    headline: row.headline,
    tags: row.tags,
    marketplace: {
      headline: row.headline,
      role_label: row.role_label,
      workflows: [],
      workflow_count: row.workflow_count,
      actions: [],
      action_count: row.action_count,
      kpis: [],
      tags: row.tags,
    },
  };
}

/** GET `/api/v1/verticals/<id>` — manifest summary plus marketplace envelope. */
export async function fetchVertical(id: string): Promise<FetchVerticalResult> {
  if (isAuthBypassed()) {
    return { vertical: seedDetail(id), source: "mock-bypass" };
  }
  try {
    const detail = await apiGet<VerticalDetail>(
      `/api/v1/verticals/${encodeURIComponent(id)}`,
    );
    return { vertical: detail, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { vertical: seedDetail(id), source: "mock-offline" };
    }
    throw err;
  }
}
