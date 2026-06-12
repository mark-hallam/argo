/**
 * Billing meter fetcher — the honest included / used / overage / cap
 * meter for the self-serve `/billing` surface (DEC-053, #2871).
 *
 * Reads `GET /api/v1/billing/meter` (handler:
 * `crates/argo-cli/src/handlers/gateway/billing_meter.rs`), which returns
 * the active tier, the bundled allowance (`included_actions`), the live
 * period-to-date action count (`used_actions`, the SAME source as
 * `/api/v1/tenant/usage` — no parallel counter), the metered overage
 * (`overage_units` = used − included, billed at `overage_multiplier`),
 * a labelled *estimate* of the overage dollars, and the effective hard
 * monthly spend cap (`cap_usd`, `null` when the tier carries none).
 *
 * NO THEATER (DEC-029): the shape below is exactly what the live endpoint
 * returns — nothing is fabricated client-side. The estimate is surfaced
 * as an estimate, never as a charged amount.
 */

import { apiGet } from "@/lib/api/client";

/** Live response from `GET /api/v1/billing/meter`. */
export interface MeterResponse {
  tier: "free" | "pro" | "enterprise";
  display_name: string;
  /** Bundled monthly allowance before overage billing begins. */
  included_actions: number;
  /** Period-to-date actions consumed. */
  used_actions: number;
  /** Metered units over the allowance (`used − included`, never < 0). */
  overage_units: number;
  /** Per-unit overage multiplier applied to the tier's base rate. */
  overage_multiplier: number;
  /** Labelled estimate of overage dollars to date (USD). */
  estimated_overage_usd: number;
  /** Effective hard monthly spend cap in USD, or `null` when none. */
  cap_usd: number | null;
  /** True when `cap_usd` is the tier default rather than a self-set cap. */
  cap_is_default: boolean;
}

/**
 * Snappy bound for the meter fetch. Comfortably above a healthy
 * gateway's sub-second answer, low enough that a stalled gateway settles
 * to the meter's honest error state in a few seconds rather than the
 * client read default (15s). Bounding the fetch is what makes the
 * "Loading usage…" perpetual spinner (shakedown F11) structurally
 * impossible — see `GatewayTimeoutError` in `lib/api/types`.
 */
export const METER_TIMEOUT_MS = 8000;

/** Fetch the active tenant's included/used/overage/cap meter. */
export function fetchBillingMeter(): Promise<MeterResponse> {
  return apiGet<MeterResponse>("/api/v1/billing/meter", {
    timeoutMs: METER_TIMEOUT_MS,
  });
}

/**
 * Percent of the bundled allowance consumed, clamped to `[0, 100]`.
 *
 * Returns `0` for an unlimited / zero allowance (`included <= 0`) and for
 * any non-finite input — there is no meaningful percentage of a zero
 * allowance, and a bar should never render a fabricated fill.
 */
export function meterUsagePercent(used: number, included: number): number {
  if (!Number.isFinite(included) || included <= 0) return 0;
  const pct = (used / included) * 100;
  if (!Number.isFinite(pct)) return 0;
  return Math.max(0, Math.min(100, pct));
}
