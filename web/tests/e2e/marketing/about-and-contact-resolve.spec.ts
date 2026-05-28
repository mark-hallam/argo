import { expect, test } from "@playwright/test";

// Regression guard for fix-marketing-about-and-contact-stub-pages.
//
// Source: e2e-ux-audit docs/skycot/ux-audits/2026-05-26-2152/
// whole-site-mvp-claims-coverage.md F7 [P1].
//
// Before this brief, /about and /contact both returned HTTP 404 —
// orphan expectations because every other site has them and the
// marketing footer linked Contact via a `mailto:` rather than a route.
// This spec walks both routes, asserts HTTP 200 + correct <title> tag,
// asserts the contact page surfaces the canonical mailto and demo
// links, and asserts the /landing footer links to both new pages so
// the next "stub footer link" regression fails CI instead of shipping.

test.describe("/about and /contact resolve and the footer links to both", () => {
  test("/about returns 200 with the correct title and an <h1>", async ({ page }) => {
    const response = await page.goto("/about", { waitUntil: "domcontentloaded" });
    expect(response, "expected a navigation response for /about").not.toBeNull();
    expect(response!.status(), "expected HTTP 200 for /about").toBe(200);
    await expect(page).toHaveTitle("About · SKYCOT");

    const heading = page.locator("h1").first();
    await expect(heading).toBeVisible();
  });

  test("/contact returns 200 with the correct title, an <h1>, and the canonical mailto + demo links", async ({
    page,
  }) => {
    const response = await page.goto("/contact", { waitUntil: "domcontentloaded" });
    expect(response, "expected a navigation response for /contact").not.toBeNull();
    expect(response!.status(), "expected HTTP 200 for /contact").toBe(200);
    await expect(page).toHaveTitle("Contact · SKYCOT");

    const heading = page.locator("h1").first();
    await expect(heading).toBeVisible();

    const mailto = page.locator('a[href^="mailto:hello@skycot.com"]').first();
    await expect(mailto).toBeVisible();

    const demoLink = page.locator('a[href="/auth/signup"]').first();
    await expect(demoLink).toBeVisible();
  });

  test("the /landing footer links to both /about and /contact", async ({ page }) => {
    await page.goto("/landing", { waitUntil: "domcontentloaded" });

    const aboutLink = page.locator('footer.mk-foot a[href="/about"]').first();
    await expect(aboutLink).toBeVisible();
    await expect(aboutLink).toHaveText("About");

    const contactLink = page.locator('footer.mk-foot a[href="/contact"]').first();
    await expect(contactLink).toBeVisible();
    await expect(contactLink).toHaveText("Contact");
  });
});
