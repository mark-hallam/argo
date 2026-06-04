/**
 * Tenant usage fetcher for the self-serve billing surface.
 *
 * Reads `GET /api/v1/tenant/usage` (handler:
 * `crates/argo-cli/src/handlers/gateway/usage_endpoint.rs`), which
 * returns the active pricing tier, the period-to-date action count,
 * the monthly cap (`max_actions_per_month`; `0` = unlimited), the
 * remaining headroom, and the next reset timestamp.
 *
 * NO THEATER: the shape below is exactly what the live endpoint
 * returns — no fabricated fields. In particular the endpoint does NOT
 * expose a worker count or `max_workers`, so the billing UI does not
 * render a worker meter (doing so would have to invent the numbers).
 */

import { apiGet } from "@/lib/api/client";

/** Live response from `GET /api/v1/tenant/usage`. */
export interface UsageResponse {
  tier: "free" | "pro" | "enterprise";
  display_name: string;
  used: number;
  /** `max_actions_per_month`; `0` means unlimited. */
  limit: number;
  /** Remaining actions, or `null` when unlimited. */
  remaining: number | null;
  /** RFC3339 timestamp of the next monthly reset. */
  resets_at: string;
}

/** Fetch the active tenant's usage + tier from the live endpoint. */
export function fetchTenantUsage(): Promise<UsageResponse> {
  return apiGet<UsageResponse>("/api/v1/tenant/usage");
}

/**
 * Percent of the monthly action cap consumed, clamped to `[0, 100]`.
 *
 * Returns `0` for the unlimited tier (`limit === 0`) and for any
 * non-finite input — there is no meaningful percentage of an
 * unlimited cap, and a bar should never render a fabricated fill.
 */
export function usagePercent(used: number, limit: number): number {
  if (!Number.isFinite(limit) || limit <= 0) return 0;
  const pct = (used / limit) * 100;
  if (!Number.isFinite(pct)) return 0;
  return Math.max(0, Math.min(100, pct));
}
