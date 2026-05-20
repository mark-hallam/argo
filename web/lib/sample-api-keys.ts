/**
 * Mock API key dataset for `/settings/api-keys`. Returned by
 * `fetchApiKeys()` when the gateway returns 404 (typical in dev with no
 * `argo gateway` running) or when the auth-bypass flag is on.
 *
 * Each entry mirrors the gateway response shape from the brief: a name,
 * a 12-char prefix preview (mask everything but the last 4), the cohort
 * + scope list, and the create / last-used / expires timestamps.
 * Anchor every timestamp to a floor-of-minute so SSR and CSR render
 * identical strings (matches `sample-team.ts` / `sample-approvals.ts`).
 */

import type { ApiKeyRow, ApiKeyScope } from "@/lib/api/api-keys";

function stableNow(): number {
  const d = new Date();
  d.setSeconds(0, 0);
  return d.getTime();
}

const NOW = stableNow();
const M = 60_000;
const H = 60 * M;
const D = 24 * H;

const READ_WORKERS: ApiKeyScope = "read:workers";
const WRITE_WORKERS: ApiKeyScope = "write:workers";
const READ_WORKFLOWS: ApiKeyScope = "read:workflows";
const RUN_WORKFLOWS: ApiKeyScope = "run:workflows";
const READ_AUDIT: ApiKeyScope = "read:audit";

export const SAMPLE_API_KEYS: ApiKeyRow[] = [
  {
    id: "key_01J0CIPIPE",
    name: "CI pipeline",
    prefix: "sk_live_a4b8...c2e1",
    scopes: [READ_WORKERS, READ_WORKFLOWS, RUN_WORKFLOWS],
    cohort_ids: ["support::trusted"],
    all_cohorts: false,
    created_at: new Date(NOW - 14 * D).toISOString(),
    last_used_at: new Date(NOW - 38 * M).toISOString(),
    expires_at: new Date(NOW + 90 * D).toISOString(),
    revoked: false,
  },
  {
    id: "key_01J0WHSYNC",
    name: "Data warehouse sync",
    prefix: "sk_live_9f2c...7ab4",
    scopes: [READ_WORKERS, READ_WORKFLOWS, READ_AUDIT],
    cohort_ids: [],
    all_cohorts: true,
    created_at: new Date(NOW - 47 * D).toISOString(),
    last_used_at: new Date(NOW - 3 * H).toISOString(),
    expires_at: null,
    revoked: false,
  },
  {
    id: "key_01J0SLACK",
    name: "Slack approvals bot",
    prefix: "sk_live_55d1...88fe",
    scopes: [READ_WORKERS, WRITE_WORKERS],
    cohort_ids: ["support::trusted", "support::developing"],
    all_cohorts: false,
    created_at: new Date(NOW - 9 * D).toISOString(),
    last_used_at: null,
    expires_at: new Date(NOW + 30 * D).toISOString(),
    revoked: false,
  },
  {
    id: "key_01J0LEGACY",
    name: "Legacy migration tool",
    prefix: "sk_live_12aa...bb99",
    scopes: [READ_AUDIT],
    cohort_ids: [],
    all_cohorts: true,
    created_at: new Date(NOW - 120 * D).toISOString(),
    last_used_at: new Date(NOW - 67 * D).toISOString(),
    expires_at: new Date(NOW - 30 * D).toISOString(),
    revoked: true,
  },
];
