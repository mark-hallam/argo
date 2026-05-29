import { expect, test } from "@playwright/test";

/**
 * JOURNEY — Prospect evaluates SKYCOT (marketing funnel).
 *
 * A buyer lands cold, compares against the tools they know, checks the
 * price, skims the docs, then clicks the primary CTA to start. This is
 * the connective marketing→product path that the atomic per-page specs
 * don't assert as one continuous walk.
 *
 * Local lane: served by the SPA on :3000 (auth-bypass + dead gateway).
 * No networkidle waits — we anchor on visible headings/nav, never on a
 * quiet network (the app shell runs permanent timers; networkidle never
 * fires — the lesson from the 2026-05-29 full-site walkthrough).
 */
test("land → compare → pricing → docs → primary CTA", async ({ page }) => {
  await page.goto("/landing");
  await expect(page.getByRole("heading", { level: 1 })).toContainText(/teammate/i);

  // Scope nav clicks to the header nav — the footer repeats the same
  // links, so an unscoped getByRole would be a strict-mode violation.
  const nav = page.locator("header.mk-top");

  // Top-nav is the journey's spine. Each hop asserts URL + a page anchor.
  await nav.getByRole("link", { name: "Compare", exact: true }).click();
  await expect(page).toHaveURL(/\/compare$/);
  await expect(page.getByRole("heading", { level: 1 })).toContainText(/workflow/i);

  await nav.getByRole("link", { name: "Pricing", exact: true }).click();
  await expect(page).toHaveURL(/\/pricing$/);
  await expect(page.getByText(/per teammate/i).first()).toBeVisible();

  await nav.getByRole("link", { name: "Docs", exact: true }).click();
  await expect(page).toHaveURL(/\/docs$/);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();

  // The "Roster" nav target is /teammates (NOT /roster) — pin it so a
  // future label/href drift that 404s the public roster is caught.
  await nav.getByRole("link", { name: "Roster", exact: true }).click();
  await expect(page).toHaveURL(/\/teammates$/);
  await expect(page.getByTestId("error-shell")).toHaveCount(0);

  // Primary CTA exists and points into the product funnel (signup/onboarding).
  const cta = page.getByRole("link", { name: /hire your first teammate/i }).first();
  await expect(cta).toBeVisible();
  const href = await cta.getAttribute("href");
  expect(href).toBeTruthy();
  expect(href).toMatch(/\/(auth|onboarding|signup)/);
});
