"use client";

/**
 * Vertical-template marketplace API client.
 *
 * The gateway exposes the loaded vertical-template registry under
 * `/api/v1/verticals` (`crates/argo-cli/src/handlers/gateway/routes_verticals_list_get.rs`).
 * The list response envelope is `{ verticals: VerticalRow[] }`; the
 * detail endpoint returns the manifest summary plus a `marketplace`
 * envelope with workflows + actions + KPIs.
 *
 * The `/api/v1/*` namespace is the only path proxied through
 * `web/next.config.ts` rewrites in dev (port 3000 → gateway 7400).
 * The legacy bare `/verticals` route still exists on the gateway but
 * is NOT proxied — calling it from the SPA hits the Next dev origin,
 * receives the SPA's own HTML 200, and JSON-parse falls through to
 * the seed fallback ("offline · 5 verticals" appearing on the camera
 * even when the gateway is alive). Use the `/api/v1` path.
 *
 * The list rows expose `skill_count` so the onboarding pick-vertical
 * wizard renders the same number that
 * `POST /api/v1/onboarding/install-vertical` will actually install.
 * Before this field landed, the wizard hard-coded counts client-side
 * and drifted from the gateway (e.g. marketplace-seller card said 6,
 * gateway installed 2).
 *
 * Acceptance contract (mirrors `lib/api/skills.ts`):
 * - 404 from the gateway → fall back to a seed catalog in dev so the
 *   marketplace landing still renders.
 * - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` → mock immediately.
 * - Strict prod (`NODE_ENV === 'production'` AND `NEXT_PUBLIC_ENV ===
 *   'production'`): no mock fallback; ApiError propagates.
 * - NetworkError (gateway unreachable) → mock in dev, propagate in prod.
 */

import { useEffect, useState } from "react";
import { hasRealSession, isDemoLive } from "@/lib/skycot/demo-mode";
import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { isMissingOrUnreachable } from "./unreachable";

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
  /**
   * Count of `*.skill.md` files in `<root>/skills/`. Matches
   * `skills_added` from `POST /api/v1/onboarding/install-vertical`,
   * so onboarding card counts never drift from what installs.
   * Optional for back-compat with older gateway builds.
   */
  skill_count?: number;
}

interface VerticalsEnvelope {
  verticals?: VerticalRow[];
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
    name: "Accounts Receivable",
    version: "0.1.0",
    description:
      "Reviews open invoices, drafts polite reminder emails on a 7/14/30-day cadence, escalates to a human at 45 days past due, logs payment receipts, and produces a weekly aging report.",
    headline:
      "Reviews open invoices, drafts polite reminder emails on a 7/14/30-day cadence, escalates to a human at 45 days past due.",
    tags: ["finance", "ar", "gmail", "stripe"],
    workflow_count: 0,
    action_count: 4,
    role_label: "Accounts Receivable",
    kpi_count: 0,
    skill_count: 5,
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
    skill_count: 8,
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
    skill_count: 5,
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
    skill_count: 5,
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
    skill_count: 5,
  },
];

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

/**
 * GET `/api/v1/verticals` — list every vertical-template the gateway
 * has loaded from disk. Post-#2456 this endpoint reads the on-disk
 * pack catalog directly via `argo_vertical::load_dir`, returning the
 * canonical `{verticals: [...]}` envelope. The path is proxied
 * through `web/next.config.ts`'s `/api/v1/*` rewrite, so the SPA
 * actually reaches the gateway (the legacy bare `/verticals` route
 * is NOT proxied and would fall back to the seed catalog).
 */
export async function fetchVerticals(): Promise<FetchVerticalsResult> {
  if (isAuthBypassed() && !isDemoLive()) {
    return { verticals: SEED_VERTICALS, source: "mock-bypass" };
  }
  try {
    const envelope = await apiGet<VerticalsEnvelope>("/api/v1/verticals");
    const verticals = envelope.verticals ?? [];
    // Success-empty fallback is for ANONYMOUS demo visitors only — a real
    // signed-in user with no verticals sees their real (empty) state, NOT
    // the SEED_VERTICALS demo fixtures (mock-leak class, path 3).
    if (verticals.length === 0 && !hasRealSession() && !isStrictProduction()) {
      return { verticals: SEED_VERTICALS, source: "mock-offline" };
    }
    return { verticals, source: "gateway" };
  } catch (err) {
    if (!hasRealSession() && !isStrictProduction() && isMissingOrUnreachable(err)) {
      return { verticals: SEED_VERTICALS, source: "mock-offline" };
    }
    throw err;
  }
}

export interface UseVerticalsState {
  verticals: VerticalRow[];
  source: VerticalDataSource;
  isLoading: boolean;
  error: Error | null;
}

/**
 * React hook for the pick-vertical wizard: fetches the gateway
 * catalog once on mount, falls back to `SEED_VERTICALS` on any
 * non-prod failure (see {@link fetchVerticals}). Returns
 * `isLoading=true` only on the initial fetch so the wizard can
 * render a skeleton without flicker if the gateway is slow.
 */
export function useVerticals(): UseVerticalsState {
  const [state, setState] = useState<UseVerticalsState>({
    verticals: [],
    source: "mock-offline",
    isLoading: true,
    error: null,
  });
  useEffect(() => {
    let cancelled = false;
    fetchVerticals()
      .then((res) => {
        if (cancelled) return;
        setState({
          verticals: res.verticals,
          source: res.source,
          isLoading: false,
          error: null,
        });
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setState({
          verticals: SEED_VERTICALS,
          source: "mock-offline",
          isLoading: false,
          error: err instanceof Error ? err : new Error(String(err)),
        });
      });
    return () => {
      cancelled = true;
    };
  }, []);
  return state;
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
  if (isAuthBypassed() && !isDemoLive()) {
    return { vertical: seedDetail(id), source: "mock-bypass" };
  }
  try {
    const detail = await apiGet<VerticalDetail>(
      `/api/v1/verticals/${encodeURIComponent(id)}`,
    );
    return { vertical: detail, source: "gateway" };
  } catch (err) {
    if (!hasRealSession() && !isStrictProduction() && isMissingOrUnreachable(err)) {
      return { vertical: seedDetail(id), source: "mock-offline" };
    }
    throw err;
  }
}
