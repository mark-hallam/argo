/**
 * v1.0 tier mapping for the operator-facing connector catalog.
 *
 * Source: round-12 audit (`docs/skycot/audit/`) + pricing-page draft
 * (`docs/skycot/marketing/PRICING.md` pending — see `mission-pricing-tier-doc-author`).
 *
 * Until the pricing doc lands, Slack / iMessage / Email (gmail) are Pro
 * — they're the SKYCOT consumer story — and everything else is
 * Enterprise. Operator-decisional changes flip a single line here.
 */
export type ConnectorTier = "pro" | "enterprise";

const PRO_TIER_IDS: ReadonlySet<string> = new Set(["slack", "imessage", "gmail"]);

export function tierFor(connectorId: string): ConnectorTier {
  return PRO_TIER_IDS.has(connectorId) ? "pro" : "enterprise";
}

export function tierLabel(tier: ConnectorTier): string {
  return tier === "pro" ? "Pro" : "Enterprise";
}
