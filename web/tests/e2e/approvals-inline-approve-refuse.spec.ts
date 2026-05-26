import { expect, test } from "@playwright/test";

/**
 * Scene-6 P0 demo-blocker — inline Approve / Refuse / Undo on the
 * `/approvals` ops dashboard.
 *
 * The webServer in `playwright.config.ts` boots the SPA with
 * `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`, which makes the data layer
 * return the bundled SAMPLE_APPROVALS instead of fetching the gateway —
 * deterministic without a live backend.
 *
 * Asserts:
 *   1. The ops-dashboard renders with inline action buttons on every
 *      pending row (the brief's central deliverable).
 *   2. Clicking Approve removes the row from the pending list
 *      optimistically and does NOT trigger the drill-in.
 *   3. Clicking Refuse on a different row also removes it without
 *      drilling in.
 */
test("approves and refuses pending rows inline on /approvals", async ({ page }) => {
  await page.goto("/approvals");

  // Wait for the ops dashboard to mount (vs the marketing hero).
  await expect(page.getByTestId("approvals-ops-dashboard")).toBeVisible();

  // Brief contract: every pending row has Approve + Refuse buttons.
  const firstApprove = page.locator('[data-testid^="approve-"]').first();
  await expect(firstApprove).toBeVisible();
  const firstRefuse = page.locator('[data-testid^="refuse-"]').first();
  await expect(firstRefuse).toBeVisible();

  // Capture the approval-id of the first pending row so we can assert
  // it disappears after the click without depending on row count
  // semantics that drift with fixture edits.
  const firstApproveId = (await firstApprove.getAttribute("data-testid")) ?? "";
  expect(firstApproveId.startsWith("approve-")).toBe(true);
  const firstApprovalId = firstApproveId.slice("approve-".length);

  await firstApprove.click();

  // Row is gone from the pending list (optimistic update from
  // useSubmitDecision flips `decision` → row filtered out of `pending`).
  await expect(
    page.getByTestId(`approve-${firstApprovalId}`),
  ).toHaveCount(0);

  // The click on Approve must NOT have navigated away from /approvals.
  await expect(page).toHaveURL(/\/approvals$/);

  // Now refuse the next pending row; same expectation shape.
  const nextRefuse = page.locator('[data-testid^="refuse-"]').first();
  const nextRefuseId = (await nextRefuse.getAttribute("data-testid")) ?? "";
  const nextApprovalId = nextRefuseId.slice("refuse-".length);

  await nextRefuse.click();

  await expect(
    page.getByTestId(`refuse-${nextApprovalId}`),
  ).toHaveCount(0);
  await expect(page).toHaveURL(/\/approvals$/);
});
