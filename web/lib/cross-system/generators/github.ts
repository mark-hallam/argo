/**
 * GitHub connector generator (v1).
 *
 * Detects issue + pull-request URLs of the form
 * `https://github.com/<owner>/<repo>/(issues|pull)/<n>` and links them
 * through verbatim. The allowlist enforces the canonical
 * `github.com` origin (no GHES support in v1).
 */
import type { GeneratorMatch, ConnectorGenerator } from "../types";

const URL_RE =
  /https:\/\/github\.com\/[A-Za-z0-9._-]{1,80}\/[A-Za-z0-9._-]{1,80}\/(?:issues|pull)\/\d{1,12}(?:#[A-Za-z0-9_-]{1,80})?/g;

export const githubGenerator: ConnectorGenerator = {
  system: "github",
  systemLabel: "GitHub",
  icon: "github",
  match(text: string): GeneratorMatch[] {
    const out: GeneratorMatch[] = [];
    for (const m of text.matchAll(URL_RE)) {
      const raw = m[0];
      const start = m.index ?? 0;
      out.push({
        start,
        end: start + raw.length,
        raw,
        deepLink: { url: raw, label: "View in GitHub" },
      });
    }
    return out;
  },
  verifyOrigin(url: string): boolean {
    return /^https:\/\/github\.com\/[A-Za-z0-9._-]+\/[A-Za-z0-9._-]+\/(issues|pull)\//.test(
      url,
    );
  },
};
