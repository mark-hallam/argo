import { expect, test } from "@playwright/test";

/**
 * Audit drill-in 404 routing (F10 [P2] from
 * `scene-6-claim-6-trust-and-approvals`).
 *
 * `<AuditActionNotFound>` is the in-shell 404 surface for an unknown
 * action id. Before this brief, demo + auth-bypass mode silently
 * fabricated a placeholder provenance card for any id missing from
 * `SAMPLE_AUDIT_DETAILS`, so the not-found shell was unreachable.
 *
 * Asserts that any id not in the seed roster lands on the not-found
 * shell — neither a fabricated detail page nor the global 500
 * boundary.
 */
test("unknown action id lands on the in-shell not-found surface", async ({
  page,
}) => {
  const actionId = "act_bogusid_does_not_exist";
  await page.goto(`/audit/action/${actionId}`);

  const notFound = page.getByTestId("audit-action-not-found");
  await expect(notFound).toBeVisible();
  await expect(notFound).toContainText(actionId);

  await expect(page.getByTestId("audit-detail-page")).toHaveCount(0);
  await expect(page.getByTestId("error-shell")).toHaveCount(0);
});

/**
 * Canonical seed ids must still render real detail — the routing
 * change does not regress known ids.
 */
test("canonical seed action id still renders the detail page", async ({
  page,
}) => {
  await page.goto("/audit/action/act_01J0JAMAL_001");

  await expect(page.getByTestId("audit-detail-page")).toBeVisible();
  await expect(page.getByTestId("audit-action-not-found")).toHaveCount(0);
});
