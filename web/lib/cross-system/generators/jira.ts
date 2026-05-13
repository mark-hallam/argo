/**
 * Jira connector generator (v1).
 *
 * Detects Atlassian Cloud issue URLs of the form
 * `https://<site>.atlassian.net/browse/<PROJECT>-<n>` and the
 * jira-software variant `https://<site>.atlassian.net/jira/...`.
 * Origin allowlist gates any `*.atlassian.net` host.
 *
 * Bare project-key detection (`<PROJECT>-<n>`) is gated by per-
 * tenant project-key configuration and out of v1 scope.
 */
import type { GeneratorMatch, ConnectorGenerator } from "../types";

const URL_RE =
  /https:\/\/[a-z0-9-]{1,63}\.atlassian\.net\/(?:browse\/[A-Z][A-Z0-9_]{1,15}-\d{1,8}|jira\/(?:software|servicedesk)\/projects\/[A-Z][A-Z0-9_]{1,15}\/(?:boards|queues)\/\d{1,8}\/?(?:[A-Z][A-Z0-9_]{1,15}-\d{1,8})?)/g;

export const jiraGenerator: ConnectorGenerator = {
  system: "jira",
  systemLabel: "Jira",
  icon: "kanban",
  match(text: string): GeneratorMatch[] {
    const out: GeneratorMatch[] = [];
    for (const m of text.matchAll(URL_RE)) {
      const raw = m[0];
      const start = m.index ?? 0;
      out.push({
        start,
        end: start + raw.length,
        raw,
        deepLink: { url: raw, label: "View in Jira" },
      });
    }
    return out;
  },
  verifyOrigin(url: string): boolean {
    return /^https:\/\/[a-z0-9-]{1,63}\.atlassian\.net\//.test(url);
  },
};
