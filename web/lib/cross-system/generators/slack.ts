/**
 * Slack connector generator (v1).
 *
 * Detects archive URLs of the form
 * `https://<workspace>.slack.com/archives/<channel>/<msg>` and links
 * them through verbatim. Workspace subdomain varies per tenant, so
 * the allowlist accepts any `*.slack.com/archives/` origin.
 */
import type { GeneratorMatch, ConnectorGenerator } from "../types";

const URL_RE =
  /https:\/\/[a-z0-9-]{1,63}\.slack\.com\/archives\/[A-Za-z0-9]{1,32}(?:\/p?\d{1,20})?(?:\?[^\s<>"']{0,200})?/g;

export const slackGenerator: ConnectorGenerator = {
  system: "slack",
  systemLabel: "Slack",
  icon: "message-square",
  match(text: string): GeneratorMatch[] {
    const out: GeneratorMatch[] = [];
    for (const m of text.matchAll(URL_RE)) {
      const raw = m[0];
      const start = m.index ?? 0;
      out.push({
        start,
        end: start + raw.length,
        raw,
        deepLink: { url: raw, label: "View in Slack" },
      });
    }
    return out;
  },
  verifyOrigin(url: string): boolean {
    return /^https:\/\/[a-z0-9-]{1,63}\.slack\.com\/archives\//.test(url);
  },
};
