import { expect, test } from "@playwright/test";

/**
 * E2-h — Step 6 celebration handoff.
 *
 * Asserts that the `/hire/done/<id>` page redirects to the approvals
 * inbox, the celebration banner surfaces there with the new worker's
 * name, and the `×` dismiss button removes the banner and the
 * underlying localStorage entry.
 *
 * The mock-fallback approvals dataset has no worker called "Quill",
 * so auto-clear-on-first-approval does not fire — the banner stays
 * visible until the user dismisses it.
 */
test("hire/done seeds the celebration and the banner appears on /", async ({
  page,
}) => {
  // Visit /hire/done directly with ?name= so we don't need a draft.
  await page.goto("/hire/done/wkr_quill?name=Quill");

  // Overlay paints briefly before the 1.5s redirect.
  await expect(page.getByTestId("hire-celebration-overlay")).toBeVisible();
  await expect(page).toHaveURL(/\/$/, { timeout: 5_000 });

  const banner = page.getByTestId("celebration-banner");
  await expect(banner).toBeVisible();
  await expect(banner).toHaveAttribute("data-variant", "active");
  await expect(banner).toContainText(/Quill is hired/);

  // Dismiss removes the banner and clears the storage entry.
  await page.getByTestId("celebration-banner-dismiss").click();
  await expect(banner).toBeHidden();
  const stored = await page.evaluate(() =>
    window.localStorage.getItem("skycot.celebrations"),
  );
  expect(stored === null || stored === "[]" || !stored.includes("wkr_quill")).toBe(true);
});

test("inert variant is rendered when ?inert=1 is set", async ({ page }) => {
  await page.goto("/hire/done/wkr_skip?name=Robin&inert=1");
  await expect(page).toHaveURL(/\/$/, { timeout: 5_000 });
  const banner = page.getByTestId("celebration-banner");
  await expect(banner).toBeVisible();
  await expect(banner).toHaveAttribute("data-variant", "inert");
  await expect(banner).toContainText(/needs tools to start working/);
  const cta = page.getByTestId("celebration-banner-cta");
  await expect(cta).toHaveAttribute(
    "href",
    /\/workers\/wkr_skip\?tab=settings/,
  );
});
