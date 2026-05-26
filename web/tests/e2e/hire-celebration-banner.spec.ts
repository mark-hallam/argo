import { expect, test } from "@playwright/test";

/**
 * Step 6 celebration handoff (post web-hire-done-celebration-brandloud).
 *
 * The `/hire/done/<id>` page no longer auto-redirects — the brand-loud
 * payoff page IS the destination, and the user chooses where to go via
 * the CTAs. The page still calls `setCelebration()` into localStorage
 * so the next visit to `/` surfaces the celebration banner above the
 * approvals inbox.
 *
 * These tests:
 *  1. Visit `/hire/done/<id>` and confirm the brand-loud payoff card
 *     paints with the worker's name + role.
 *  2. Navigate to `/`, where the dismissible celebration banner
 *     surfaces with the right copy + CTA.
 *  3. Cover both the active and inert (`?inert=1`) variants.
 */
test("hire/done paints the brand-loud payoff, and the banner appears on /", async ({
  page,
}) => {
  // Visit /hire/done with ?name= so we don't need a draft.
  await page.goto("/hire/done/wkr_quill?name=Quill");
  await expect(page.getByTestId("hire-done-welcome-card")).toBeVisible();
  await expect(page.getByTestId("hire-done-welcome-card")).toContainText("Quill");

  // The user navigates home; the banner surfaces with the active
  // variant.
  await page.goto("/");
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
  await expect(page.getByTestId("hire-done-welcome-card")).toBeVisible();
  await expect(page.getByTestId("hire-done-welcome-card")).toContainText("Robin");

  await page.goto("/");
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
