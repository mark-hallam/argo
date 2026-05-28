import { expect, test } from "@playwright/test";

/**
 * Regression guard for rebase-fix-settings-workspace-tenant-identity-surface.
 *
 * Source: ux-audits/2026-05-26-2152 scene-6-claim-6-trust-and-approvals F7.
 * The /settings/workspace page must surface the live `tenantId` from the
 * session, and the sidebar UserPill popover must render the tenant chip
 * beneath the email line. Both are demo-visible signals for Claim 6
 * (multi-channel + multi-tenant) — without them a buyer can't see that
 * SKYCOT is multi-tenant.
 *
 * Runs against demo mode (`NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`), the only
 * branch that hydrates a deterministic tenant id without the gateway in
 * the loop. Skips itself when demo mode is off.
 */

test.skip(
  process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1",
  "requires NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 to render a deterministic tenant id",
);

test("/settings/workspace shows the Workspace ID row with a non-empty value", async ({
  page,
}) => {
  await page.goto("/settings/workspace", { waitUntil: "domcontentloaded" });

  const row = page.getByTestId("settings-workspace-tenant-id-row");
  await expect(row).toBeVisible();

  const value = page.getByTestId("settings-workspace-tenant-id-value");
  await expect(value).toBeVisible();
  const text = (await value.textContent())?.trim() ?? "";
  expect(text.length, `Workspace ID value should be non-empty: ${text}`).toBeGreaterThan(0);
  expect(text, "Workspace ID should not be the em-dash fallback").not.toBe("—");

  await expect(page.getByTestId("settings-workspace-tenant-id-copy")).toBeEnabled();
});

test("sidebar UserPill popover surfaces the tenant chip", async ({ page }) => {
  await page.goto("/settings/workspace", { waitUntil: "domcontentloaded" });

  await page.getByTestId("appshell-user-menu-trigger").click();

  const chip = page.getByTestId("appshell-user-tenant-chip");
  await expect(chip).toBeVisible();
  const chipText = (await chip.textContent())?.trim() ?? "";
  expect(chipText, `tenant chip should be non-empty: ${chipText}`).toMatch(/tenant_/);
});
