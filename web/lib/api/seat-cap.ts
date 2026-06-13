/**
 * Seat-cap (pricing-tier `max_workers`) 403 detection + upgrade copy.
 *
 * The gateway's hire path
 * (`crates/argo-cli/src/handlers/gateway/tier_gate.rs::worker_cap_response`)
 * refuses a hire that would exceed the tenant's pricing-tier worker cap
 * with a structured `403 { "error": "tier_worker_cap", "tier", "used",
 * "max_workers", "tier_upsell_url" }`. `lib/api/client.ts::parseError`
 * surfaces that flat `error` string as `ApiError.code` and keeps the parsed
 * body, so the hire UI can branch a friendly "you hit your plan limit"
 * upsell off what would otherwise render as a bare, scary "Forbidden" auth
 * error. A generic RBAC `forbidden` 403 is deliberately NOT matched here, so
 * it still surfaces the honest auth error.
 *
 * See brief web-seat-cap-403-upgrade-prompt-not-bare-forbidden.
 */
import { ApiError } from "./types";
import { PRO } from "@/lib/pricing/tiers";

/** Gateway error code for the tier worker-seat cap. */
export const SEAT_CAP_ERROR_CODE = "tier_worker_cap";

/** In-product upgrade surface the prompt links to. */
export const SEAT_CAP_UPGRADE_PATH = "/billing";

export interface SeatCapInfo {
  /** Tenant's current pricing tier (e.g. "free"). */
  tier: string;
  /** Workers already used, when the gateway reported it. */
  used?: number;
  /** The `max_workers` cap the hire would have exceeded. */
  maxWorkers: number;
  /** Absolute tier-upsell URL the gateway pointed at, when present. */
  upsellUrl?: string;
}

/**
 * Returns `SeatCapInfo` when `err` is the seat-cap 403, else `null`.
 * Matches ONLY the `tier_worker_cap` code so a generic `forbidden` RBAC
 * 403 keeps its honest auth error.
 */
export function seatCapErrorInfo(err: unknown): SeatCapInfo | null {
  if (!(err instanceof ApiError)) return null;
  if (err.status !== 403 || err.code !== SEAT_CAP_ERROR_CODE) return null;
  const body =
    err.body && typeof err.body === "object"
      ? (err.body as Record<string, unknown>)
      : {};
  return {
    tier: typeof body.tier === "string" ? body.tier : "free",
    used: typeof body.used === "number" ? body.used : undefined,
    maxWorkers: typeof body.max_workers === "number" ? body.max_workers : 1,
    upsellUrl:
      typeof body.tier_upsell_url === "string" ? body.tier_upsell_url : undefined,
  };
}

/** Title-case a bare tier id ("free" → "Free", "pro" → "Pro"). */
function tierLabel(tier: string): string {
  if (!tier) return "Your";
  return tier.charAt(0).toUpperCase() + tier.slice(1);
}

/** "1 teammate" / "5 teammates". */
function teammates(n: number): string {
  return `${n} ${n === 1 ? "teammate" : "teammates"}`;
}

/**
 * On-brand upgrade copy for the seat-cap prompt, e.g.
 * "You've reached your Free plan's 1-teammate limit — upgrade to Pro to
 * hire up to 5." The current limit comes from the gateway's structured
 * error; the upgrade target (Pro cap) reads from the canonical pricing
 * source (`lib/pricing/tiers.ts`) so the number can't drift from /pricing.
 */
export function seatCapMessage(info: SeatCapInfo): string {
  // "1-teammate limit" / "5-teammate limit" — the compound adjective stays
  // singular regardless of the count.
  const base = `You've reached your ${tierLabel(info.tier)} plan's ${info.maxWorkers}-teammate limit`;
  // A Pro/Enterprise tenant already at cap shouldn't be told to "upgrade to
  // Pro" — point them at adding seats instead.
  if (info.tier === "pro" || info.tier === "enterprise") {
    return `${base} — contact us to add more seats.`;
  }
  return `${base} — upgrade to Pro to hire up to ${teammates(PRO.max_workers)}.`;
}
