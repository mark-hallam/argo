/**
 * Zendesk connector generator (v1).
 *
 * Detects support-ticket URLs of the form
 * `https://<subdomain>.zendesk.com/agent/tickets/<n>`. Subdomain is
 * customer-specific so the allowlist accepts any `*.zendesk.com`
 * host that targets the `/agent/tickets/` path.
 *
 * Bare-ID detection (`ZD-<n>`) is out of v1 scope per the brief.
 */
import type { GeneratorMatch, ConnectorGenerator } from "../types";

const URL_RE =
  /https:\/\/[a-z0-9-]{1,63}\.zendesk\.com\/agent\/tickets\/\d{1,12}/g;

export const zendeskGenerator: ConnectorGenerator = {
  system: "zendesk",
  systemLabel: "Zendesk",
  icon: "life-buoy",
  match(text: string): GeneratorMatch[] {
    const out: GeneratorMatch[] = [];
    for (const m of text.matchAll(URL_RE)) {
      const raw = m[0];
      const start = m.index ?? 0;
      out.push({
        start,
        end: start + raw.length,
        raw,
        deepLink: { url: raw, label: "View in Zendesk" },
      });
    }
    return out;
  },
  verifyOrigin(url: string): boolean {
    return /^https:\/\/[a-z0-9-]{1,63}\.zendesk\.com\/agent\/tickets\//.test(
      url,
    );
  },
};
