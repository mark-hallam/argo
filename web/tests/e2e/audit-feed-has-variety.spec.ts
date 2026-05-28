import { expect, test } from "@playwright/test";

/**
 * SPA regression pin for the audit-feed variety fix.
 *
 * Brief: `docs/build-plan/streams/rebase-fix-audit-feed-action-kind-variety.md`.
 *
 * Before the fix, `/audit` rendered 13 visible rows all sharing
 * `action_type: http.request` from actor `UN` — the gateway's own
 * `audit.feed.queried` self-traffic, leaking through because there
 * was no `include_meta` filter and no varied-action-type backstory
 * seed.
 *
 * This spec pins three invariants over the rows the SPA renders on
 * first paint of `/audit`:
 *
 *  1. ≥5 distinct `action_type` values across the visible rows.
 *  2. Zero meta rows (`http.request`, `audit.detail.viewed`,
 *     `audit.feed.queried`, `worker.kpi_rolled`, etc.) — the gateway
 *     `include_meta` filter and the SPA's day-grouping share the
 *     same `argo_audit::METADATA_VERBS` list.
 *  3. ≥3 distinct worker ids attributed to the visible rows —
 *     guards against the regression where the feed is dominated by
 *     a single actor.
 *
 * The Playwright config bypasses the live gateway, so this spec runs
 * against the bundled SPA mock dataset (`web/lib/sample-audit.ts`).
 * That dataset already has wide actionType variety; the spec
 * therefore acts as a regression pin against any future change that
 * collapses the visible mix back to a single label.
 */

const METADATA_VERBS = [
  "http.request",
  "worker.kpi_rolled",
  "audit.list.viewed",
  "audit.detail.viewed",
  "audit.replay",
  "worker.activity.viewed",
  "audit.feed.queried",
];

test("/audit default view has ≥5 distinct action types and no meta rows", async ({
  page,
}) => {
  await page.goto("/audit?range=30d");

  const rows = page.locator("[data-testid^='audit-row-']");
  await expect(rows.first()).toBeVisible();

  // Sample the first 20 rows the SPA renders (or however many it
  // shows — whichever is smaller).
  const count = Math.min(await rows.count(), 20);
  expect(count).toBeGreaterThanOrEqual(5);

  const actionTypes: string[] = [];
  const workerIds: string[] = [];
  for (let i = 0; i < count; i += 1) {
    const row = rows.nth(i);
    const actionType = await row.getAttribute("data-action-type");
    const workerId = await row.getAttribute("data-worker-id");
    if (actionType) actionTypes.push(actionType);
    if (workerId) workerIds.push(workerId);
  }

  const distinctActionTypes = new Set(actionTypes);
  expect(
    distinctActionTypes.size,
    `expected ≥5 distinct action_type values in first ${count} rows, got ${[...distinctActionTypes].join(", ")}`,
  ).toBeGreaterThanOrEqual(5);

  const metaHits = actionTypes.filter((t) => METADATA_VERBS.includes(t));
  expect(
    metaHits,
    `default /audit view must not include meta rows (got ${metaHits.join(", ")})`,
  ).toEqual([]);

  const distinctWorkers = new Set(workerIds.filter((w) => w.length > 0));
  expect(
    distinctWorkers.size,
    `expected ≥3 distinct workers attributed to the feed (got ${[...distinctWorkers].join(", ")})`,
  ).toBeGreaterThanOrEqual(3);
});
