import { expect, test } from "@playwright/test";

/**
 * Operator-vocabulary contract — "Duty" / "Duties" are internal-only
 * words; operator-visible SPA copy uses "Schedule" / "Schedules".
 *
 * Locked 2026-05-23 via brief
 * `streams/web-rename-duty-to-schedule-in-ui-copy.md` and reinforced
 * 2026-05-26 via the chrome-live-walk-and-playwright audit's
 * `Vocabulary Duty → Schedule` cluster. Source: operator memory
 * `project_vocabulary_duty_vs_schedule`. The negative side of the
 * contract walks the 8 in-product index routes operators land on
 * during the demo runbook; the positive side proves the rename
 * actually surfaced "Schedule" copy where the concept lives (the
 * worker-profile tab strip).
 *
 * The check inspects `document.body.innerText` — i.e. what an
 * operator can actually read — and ignores test-ids, comments, code
 * identifiers, aria-labels, and API paths. The internal vocabulary
 * (`duty`/`duties` in code, SQL, audit kinds, brief filenames, and
 * testid attributes) is intentionally preserved.
 */

const FORBIDDEN_PAGES: Array<{ name: string; path: string }> = [
  { name: "settings", path: "/settings" },
  { name: "workflows", path: "/workflows" },
  { name: "cohorts", path: "/cohorts" },
  { name: "workers", path: "/workers" },
  { name: "audit", path: "/audit" },
  { name: "approvals", path: "/approvals" },
  { name: "skills", path: "/skills" },
  { name: "hire", path: "/hire" },
];

const FORBIDDEN = /\b(dut(y|ies))\b/i;

for (const { name, path } of FORBIDDEN_PAGES) {
  test(`${name} page contains no operator-visible "Duty" / "Duties" copy`, async ({
    page,
  }) => {
    await page.goto(path);
    await expect(page.locator("body")).toBeVisible();
    const innerText = await page.evaluate(() => document.body.innerText);
    expect(innerText, `${name} page surfaced a forbidden word`).not.toMatch(
      FORBIDDEN,
    );
  });
}

/**
 * Positive complement — guards against the easy regression of the
 * forbidden-word check passing simply because every "Schedule" string
 * vanished alongside the "Duty" ones. The worker-profile route is the
 * canonical home for the renamed concept: the tab strip surfaces the
 * "Schedules" label on every sub-route (overview / activity / etc.)
 * via `WorkerTabs`, so any cold nav to `/workers/<id>` is enough.
 *
 * Mariah is the first persona in `lib/sample-personas.ts` and is the
 * shared "demo worker" used across other Playwright specs
 * (`worker-detail-cold-nav.spec.ts`). Using her id keeps the spec
 * deterministic under the mock-fallback lane.
 */
const MARIAH_ID = "019e6785-928a-7270-a766-f945b6cd7161";

test('worker profile surfaces "Schedules" copy after the duty→schedule rename', async ({
  page,
}) => {
  await page.goto(`/workers/${MARIAH_ID}/overview`);
  // Cold nav can flash a LoadingState skeleton before the
  // `useWorker` query resolves against the sample-data fallback —
  // wait for the hero so `WorkerTabs` (which only mounts after the
  // worker loads) is in the DOM before sampling `innerText`.
  await expect(page.getByTestId("worker-name")).toHaveText(/mariah/i);
  const innerText = await page.evaluate(() => document.body.innerText);
  expect(
    /\b(schedule|schedules)\b/i.test(innerText),
    'worker profile should surface "Schedules" copy after the duty→schedule rename',
  ).toBe(true);
});
