/**
 * Linear connector generator (v1).
 *
 * Detects Linear issue URLs of the form
 * `https://app.linear.app/<workspace>/issue/<TEAM>-<n>/...`. Bare-ID
 * detection (`SKY-123` without surrounding URL) is gated by per-
 * tenant team-prefix configuration and is therefore out of scope for
 * v1.
 */
import type { GeneratorMatch, ConnectorGenerator } from "../types";

const URL_RE =
  /https:\/\/app\.linear\.app\/[A-Za-z0-9._-]{1,80}\/issue\/[A-Z]{2,12}-\d{1,12}(?:\/[^\s<>"']{0,160})?/g;

export const linearGenerator: ConnectorGenerator = {
  system: "linear",
  systemLabel: "Linear",
  icon: "list-todo",
  match(text: string): GeneratorMatch[] {
    const out: GeneratorMatch[] = [];
    for (const m of text.matchAll(URL_RE)) {
      const raw = m[0];
      const start = m.index ?? 0;
      out.push({
        start,
        end: start + raw.length,
        raw,
        deepLink: { url: raw, label: "View in Linear" },
      });
    }
    return out;
  },
  verifyOrigin(url: string): boolean {
    return url.startsWith("https://app.linear.app/");
  },
};
