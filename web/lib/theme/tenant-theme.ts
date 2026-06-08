/**
 * Per-tenant white-label theme override client.
 *
 * Fetches a tenant's persisted accent override from the gateway
 * (`GET /api/v1/tenant/theme`) and renders it into a `data-theme`-scoped
 * CSS snippet by REUSING {@link synthesizeTenantCss} — the single
 * tenant-theme synthesizer. We never reimplement the palette math here.
 *
 * ## Brand guardrail (DEC-027)
 *
 * The SKYCOT brand is LOCKED. The override is *additive*: when the
 * gateway reports `is_override: false` (no row persisted), this module
 * yields an EMPTY CSS string and the SPA keeps the shipped brand tokens
 * untouched. Only a tenant that has explicitly saved an accent gets a
 * synthesized palette, emitted under the dedicated `tenant-light` /
 * `tenant-dark` data-theme ids so it never overwrites the locked
 * `skycot-*` token blocks.
 */

import { apiGet } from "@/lib/api/client";
import { hasRealSession, isDemoLive } from "@/lib/skycot/demo-mode";
import { isMissingOrUnreachable } from "@/lib/api/unreachable";
import { synthesizeTenantCss, type TenantThemeInput } from "@/lib/theme/synthesize";

/**
 * The `data-theme` id family the override is emitted under. Matches the
 * `TENANT_OVERRIDE_THEME` pair registered in `themes.ts`, so selecting
 * that theme makes the injected CSS apply.
 */
export const TENANT_THEME_ID = "tenant";

/** Shape returned by `GET /api/v1/tenant/theme` (snake_case from Rust). */
export interface TenantThemeOverride {
  accent_hex: string;
  neutral_tint: "cool" | "neutral" | "warm";
  radius: "sharp" | "default" | "rounded";
  /** `true` once a tenant has persisted an accent; `false` = locked default. */
  is_override: boolean;
}

/** The locked SKYCOT default, mirroring the gateway `default_theme()`. */
export const SKYCOT_DEFAULT_OVERRIDE: TenantThemeOverride = {
  accent_hex: "#8B5CF6",
  neutral_tint: "cool",
  radius: "default",
  is_override: false,
};

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  // Mirrors `lib/api/settings.ts`: a REAL signed-in user is never bypassed
  // (they read their own persisted override); anonymous demo dev mocks.
  if (hasRealSession()) return false;
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

/**
 * Fetch the caller-tenant's theme override. Returns the locked SKYCOT
 * default (`is_override: false`) for anonymous demo dev, on a 404 / network
 * error in non-strict builds, or whenever the gateway reports no override —
 * so a missing override NEVER white-labels the brand.
 */
export async function fetchTenantThemeOverride(): Promise<TenantThemeOverride> {
  if (isAuthBypassed() && !isDemoLive()) return SKYCOT_DEFAULT_OVERRIDE;
  try {
    return await apiGet<TenantThemeOverride>("/api/v1/tenant/theme");
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return SKYCOT_DEFAULT_OVERRIDE;
    }
    throw err;
  }
}

/**
 * Render a fetched override into a `data-theme`-scoped CSS snippet via
 * {@link synthesizeTenantCss}. Returns `""` when the override is absent
 * (`is_override: false`), so the caller injects nothing and the locked
 * brand tokens stand.
 */
export function tenantOverrideCss(override: TenantThemeOverride): string {
  if (!override.is_override) return "";
  const input: TenantThemeInput = {
    id: TENANT_THEME_ID,
    accentHex: override.accent_hex,
    neutralTint: override.neutral_tint,
    radius: override.radius,
  };
  return synthesizeTenantCss(input);
}
