/**
 * Deterministic sample data for the workflow runs summary endpoint.
 *
 * Used by `fetchWorkflowRuns` in two cases:
 *   1. The user is in dev mode and the gateway 404s (`mock-offline`).
 *   2. `ARGO_GATEWAY_AUTH_BYPASS=1` (`mock-bypass`).
 *
 * Strict production (`NODE_ENV=production` + `NEXT_PUBLIC_ENV=production`)
 * never falls back here — the surface returns the real gateway error
 * instead so the demo recording fails loudly rather than masking a
 * broken endpoint with stale numbers.
 *
 * The summary is varied per workflow via a tiny deterministic hash on
 * `(verticalId, name)` so different workflows show different counts
 * in dev / preview without needing a populated database. The recent-
 * runs array is a small 10-row sample so the demo table is never
 * empty in mock mode.
 */

export interface SampleWorkflowRun {
  id: string;
  started_at: string;
  finished_at: string | null;
  ok: boolean;
  duration_ms: number | null;
}

export interface SampleWorkflowRunsSummary {
  vertical_id: string;
  workflow_name: string;
  window_days: number;
  run_count: number;
  success_rate: number | null;
  avg_duration_ms: number;
  p95_duration_ms: number;
  last_run_at: string | null;
  runs: SampleWorkflowRun[];
}

const BASE_RUNS: SampleWorkflowRun[] = [
  { id: "wfr_01HXY7K5G2", started_at: "2026-05-20T14:32:00Z", finished_at: "2026-05-20T14:32:04Z", ok: true,  duration_ms: 4321 },
  { id: "wfr_01HXY6J9F1", started_at: "2026-05-20T11:18:00Z", finished_at: "2026-05-20T11:18:09Z", ok: true,  duration_ms: 9015 },
  { id: "wfr_01HXY3M2E8", started_at: "2026-05-19T22:04:00Z", finished_at: "2026-05-19T22:04:03Z", ok: true,  duration_ms: 3142 },
  { id: "wfr_01HXY1N4C2", started_at: "2026-05-19T16:51:00Z", finished_at: "2026-05-19T16:51:12Z", ok: false, duration_ms: 12543 },
  { id: "wfr_01HXX0Q7B5", started_at: "2026-05-19T09:27:00Z", finished_at: "2026-05-19T09:27:04Z", ok: true,  duration_ms: 4202 },
  { id: "wfr_01HXW9R3A1", started_at: "2026-05-18T19:14:00Z", finished_at: "2026-05-18T19:14:05Z", ok: true,  duration_ms: 5071 },
  { id: "wfr_01HXW5S6D4", started_at: "2026-05-18T13:02:00Z", finished_at: "2026-05-18T13:02:03Z", ok: true,  duration_ms: 3499 },
  { id: "wfr_01HXV2T8E9", started_at: "2026-05-18T08:48:00Z", finished_at: "2026-05-18T08:48:06Z", ok: true,  duration_ms: 6188 },
  { id: "wfr_01HXU4U1F6", started_at: "2026-05-17T20:33:00Z", finished_at: "2026-05-17T20:33:04Z", ok: true,  duration_ms: 4615 },
  { id: "wfr_01HXT8V7H3", started_at: "2026-05-17T14:09:00Z", finished_at: "2026-05-17T14:09:05Z", ok: true,  duration_ms: 5290 },
];

/**
 * Tiny FNV-1a-style hash so the same (verticalId, name) pair always
 * returns the same `run_count`. Good enough for "make the demo look
 * varied per workflow" — not a security-grade hash.
 */
function fingerprint(verticalId: string, name: string): number {
  let h = 0x811c9dc5;
  for (const ch of `${verticalId}/${name}`) {
    h ^= ch.charCodeAt(0);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h;
}

export function sampleWorkflowRunsSummary(
  verticalId: string,
  name: string,
  days = 30,
): SampleWorkflowRunsSummary {
  const fp = fingerprint(verticalId, name);
  // Span 12 → 187 so different workflows show distinct numbers, never
  // landing on the literal `82` the brief is replacing.
  const runCount = 12 + (fp % 176);
  // Success rate spans 86% → 97% deterministically.
  const successRate = 0.86 + ((fp >>> 8) % 12) / 100;
  return {
    vertical_id: verticalId,
    workflow_name: name,
    window_days: days,
    run_count: runCount,
    success_rate: successRate,
    avg_duration_ms: 4321,
    p95_duration_ms: 12543,
    last_run_at: BASE_RUNS[0]?.started_at ?? null,
    runs: BASE_RUNS,
  };
}

/** The canonical "looks populated" fixture used by tests + Storybook. */
export const SAMPLE_WORKFLOW_RUNS_SUMMARY: SampleWorkflowRunsSummary =
  sampleWorkflowRunsSummary("pr_checker", "standard_review");
