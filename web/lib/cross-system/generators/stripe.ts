/**
 * Stripe connector generator (v1).
 *
 * Detects two reference forms:
 *   1. Dashboard URLs: `https://dashboard.stripe.com/<type>/<id>` —
 *      always linked.
 *   2. Bare object IDs (`re_<id>`, `ch_<id>`, `cus_<id>`, `in_<id>`,
 *      `pi_<id>`, `pm_<id>`) — linked only when preceded within 80
 *      characters by a Stripe-context keyword. The conservative
 *      gate avoids false positives on incidental tokens like
 *      `re_test` appearing in chat snippets.
 *
 * v1.5 (`mission-enterprise-connector-deep-link-manifest`) replaces
 * this with a manifest declaration that declares the prefix table
 * + URL template + verify-origin allowlist.
 */
import type { GeneratorMatch, ConnectorGenerator } from "../types";

/** Stripe object-id prefix → dashboard subpath. */
const PREFIX_PATHS: Record<string, string> = {
  re_: "refunds",
  ch_: "payments",
  cus_: "customers",
  in_: "invoices",
  pi_: "payments",
  pm_: "payment-methods",
};

const PREFIX_LABELS: Record<string, string> = {
  re_: "Refund",
  ch_: "Charge",
  cus_: "Customer",
  in_: "Invoice",
  pi_: "Payment intent",
  pm_: "Payment method",
};

const URL_RE = /https:\/\/dashboard\.stripe\.com\/[A-Za-z0-9/_-]{1,200}/g;
const BARE_ID_RE = /\b(re_|ch_|cus_|in_|pi_|pm_)([A-Za-z0-9]{8,64})\b/g;
const CONTEXT_KEYWORDS = /stripe|refund|payment|charge|invoice|customer/i;
const CONTEXT_WINDOW = 80;

export const stripeGenerator: ConnectorGenerator = {
  system: "stripe",
  systemLabel: "Stripe",
  icon: "credit-card",
  match(text: string): GeneratorMatch[] {
    const out: GeneratorMatch[] = [];
    for (const m of text.matchAll(URL_RE)) {
      const raw = m[0];
      const start = m.index ?? 0;
      out.push({
        start,
        end: start + raw.length,
        raw,
        deepLink: { url: raw, label: "View in Stripe" },
      });
    }
    for (const m of text.matchAll(BARE_ID_RE)) {
      const raw = m[0];
      const start = m.index ?? 0;
      const end = start + raw.length;
      if (overlapsRanges(out, start, end)) continue;
      const windowStart = Math.max(0, start - CONTEXT_WINDOW);
      const context = text.slice(windowStart, start);
      if (!CONTEXT_KEYWORDS.test(context)) continue;
      const prefix = m[1];
      const subpath = PREFIX_PATHS[prefix];
      const objectLabel = PREFIX_LABELS[prefix];
      if (!subpath || !objectLabel) continue;
      out.push({
        start,
        end,
        raw,
        deepLink: {
          url: `https://dashboard.stripe.com/${subpath}/${raw}`,
          label: `View ${objectLabel.toLowerCase()} in Stripe`,
        },
      });
    }
    return out.sort((a, b) => a.start - b.start);
  },
  verifyOrigin(url: string): boolean {
    return url.startsWith("https://dashboard.stripe.com/");
  },
};

function overlapsRanges(
  ranges: GeneratorMatch[],
  start: number,
  end: number,
): boolean {
  for (const r of ranges) {
    if (start < r.end && end > r.start) return true;
  }
  return false;
}
