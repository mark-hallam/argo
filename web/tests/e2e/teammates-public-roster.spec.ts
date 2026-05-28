import { expect, test } from "@playwright/test";

/**
 * Regression guard for fix-landing-roster-unauth-fallback-not-team-shell.
 *
 * Source: e2e-ux-audit docs/skycot/ux-audits/2026-05-26-2152/
 * whole-site-mvp-claims-coverage.md F6 [P1].
 *
 * Before this brief, /landing's top-nav "Roster" link pointed at
 * /team — a route under the (app) group that renders the in-app
 * shell (sidebar, demo banner, empty-state CTA) regardless of
 * session. Logged-out marketing visitors saw a half-broken
 * authenticated surface. This brief introduces /teammates as a
 * public marketing-mirror and re-points the nav link.
 *
 * The spec runs without authenticating — Playwright's default
 * context carries no session cookie — and asserts:
 *  - /teammates returns HTTP 200
 *  - the canonical day-1 personas (Mariah, Peta, Priya, Jamal, Nori)
 *    are all rendered
 *  - the in-app sidebar nav + DemoBanner are absent
 *  - the CTA points at /auth/signup
 */

const PERSONA_NAMES = ["Mariah", "Peta", "Priya", "Jamal", "Nori"] as const;

test.describe("/teammates public marketing roster", () => {
  test("returns 200 for logged-out visitors and renders the five canonical personas", async ({
    page,
  }) => {
    const response = await page.goto("/teammates", {
      waitUntil: "domcontentloaded",
    });
    expect(response, "expected a navigation response for /teammates").not.toBeNull();
    expect(response!.status(), "expected HTTP 200 for /teammates").toBe(200);

    const heading = page.getByRole("heading", { level: 1 }).first();
    await expect(heading).toBeVisible();
    await expect(heading).toHaveText(/meet the team you.?ll hire/i);

    for (const name of PERSONA_NAMES) {
      await expect(
        page.getByRole("heading", { name, level: 2 }),
      ).toBeVisible();
    }
  });

  test("renders no in-app shell — sidebar nav and demo banner are absent", async ({
    page,
  }) => {
    await page.goto("/teammates", { waitUntil: "domcontentloaded" });

    // The in-app shell renders a <nav> with aria-label "Primary".
    // The marketing nav uses aria-label "Marketing".
    await expect(page.getByRole("navigation", { name: "Primary" })).toHaveCount(
      0,
    );
    await expect(
      page.getByRole("navigation", { name: "Marketing" }),
    ).toBeVisible();

    // DemoBanner exposes data-testid="demo-banner" when mounted.
    await expect(page.getByTestId("demo-banner")).toHaveCount(0);
  });

  test("CTA links to /auth/signup", async ({ page }) => {
    await page.goto("/teammates", { waitUntil: "domcontentloaded" });
    const cta = page.getByTestId("teammates-cta");
    await expect(cta).toBeVisible();
    await expect(cta).toHaveAttribute("href", "/auth/signup");
  });

  test("MarketingNav 'Roster' link points at /teammates", async ({ page }) => {
    await page.goto("/landing", { waitUntil: "domcontentloaded" });
    const desktopNav = page.getByRole("navigation", { name: "Marketing" });
    const rosterLink = desktopNav.getByRole("link", { name: "Roster" });
    await expect(rosterLink).toBeVisible();
    await expect(rosterLink).toHaveAttribute("href", "/teammates");
  });
});
