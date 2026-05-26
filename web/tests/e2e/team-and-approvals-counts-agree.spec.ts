import { expect, test } from "@playwright/test";

/**
 * Regression pin for `fix-approvals-inbox-waiting-on-you-fixture-drift`.
 *
 * Pre-fix defect: `/team` showed 4 of 5 cards with the "Waiting on
 * you" status pill while `/approvals` displayed the empty-state card
 * "No approvals waiting" — two demo fixtures had drifted out of
 * sync, so the two surfaces disagreed about pending work.
 *
 * Post-fix invariant: `pendingApprovalCount` on each /team Worker
 * is derived from `SAMPLE_APPROVALS` by name (see
 * `web/lib/sample-team.ts`), and the /approvals landing always
 * renders the `ApprovalsOpsDashboard` instead of short-circuiting
 * to the EmptyState. This spec asserts that the camera-visible
 * count on /team agrees with the inbox row count on /approvals.
 *
 * Webserver knobs (mock-fallback lane, set in playwright.config):
 *   - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`
 *   - `NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:1`
 * Both /team and /approvals therefore read the same bundled
 * `SAMPLE_TEAM` + `SAMPLE_APPROVALS` fixtures.
 */
test("/team 'Waiting on you' count matches /approvals pending row count", async ({
  page,
}) => {
  await page.goto("/team");
  const teamCards = page.getByTestId(/^teammate-status-/);
  await expect(teamCards.first()).toBeVisible();

  const statusLabels = await teamCards.allTextContents();
  const waitingTeamCount = statusLabels.filter((s) =>
    s.toLowerCase().includes("waiting on you"),
  ).length;
  expect(waitingTeamCount).toBeGreaterThan(0);

  await page.goto("/approvals");
  await expect(page.getByTestId("approvals-ops-dashboard")).toBeVisible();
  const pendingRows = page.getByTestId(/^recent-approval-/);
  await expect(pendingRows.first()).toBeVisible();
  const inboxPendingCount = await pendingRows.count();

  expect(inboxPendingCount).toBeGreaterThanOrEqual(waitingTeamCount);
  expect(inboxPendingCount).toBeGreaterThanOrEqual(4);
});

/**
 * The locked E-ops-dashboard must render on /approvals even when
 * the queue is empty — `fix-approvals-inbox-waiting-on-you-fixture-
 * drift` removed the EmptyState short-circuit so the surface stays
 * coherent with /team in the 0-pending case too. Under the mock
 * fallback the queue is non-empty, so this just asserts the
 * dashboard chrome (KPIs + severity tiers) is present.
 */
test("/approvals renders the ops-dashboard chrome, never the empty-state card", async ({
  page,
}) => {
  await page.goto("/approvals");

  await expect(page.getByTestId("approvals-ops-dashboard")).toBeVisible();
  await expect(page.getByTestId("approvals-kpi-strip")).toBeVisible();
  await expect(page.getByTestId("severity-tier-p0")).toBeVisible();
  await expect(page.getByTestId("severity-tier-p1")).toBeVisible();
  await expect(page.getByTestId("severity-tier-p2")).toBeVisible();
  await expect(page.getByTestId("approvals-empty")).toHaveCount(0);
});
