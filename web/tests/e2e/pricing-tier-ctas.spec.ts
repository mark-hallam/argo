import { expect, test, type Page } from "@playwright/test";

/**
 * `/pricing` per-tier CTAs map to tier-specific destinations so the
 * public conversion funnel doesn't lose tier intent at the signup
 * boundary.
 *
 * Brief: fix-pricing-page-tier-cards-link-to-subscribe.
 * Source: e2e-ux-audit
 *   docs/skycot/ux-audits/2026-05-27-0415/whole-site-mvp-claims-coverage.md
 *   F10 [P2].
 *
 * Signed-out: Pro CTA → `/auth/signup?plan=pro` (selectedPlan is also
 * persisted to the onboarding draft so the welcome step can hop to
 * checkout).
 *
 * Signed-in: Pro CTA → `/billing/subscribe?plan=pro`.
 *
 * Enterprise CTA: stays on a talk-to-sales surface (mailto in v1.0;
 * `/contact` form arrives with the CRM-integration brief).
 *
 * Featured override: `?featured=pro` lights up the Pro card even
 * if a future config un-features it.
 */

const DRAFT_STORAGE_KEY = "skycot:onboarding-draft-v1";

test.describe("pricing tier CTAs", () => {
  test("signed-out Pro CTA deep-links to /auth/signup?plan=pro and persists plan", async ({
    page,
  }) => {
    await stubUnauthenticated(page);
    await page.goto("/pricing");

    const proCta = page.getByTestId("pricing-cta-pro");
    await expect(proCta).toBeVisible();
    await expect(proCta).toHaveText(/Choose Pro/);
    await expect(proCta).toHaveAttribute("href", "/auth/signup?plan=pro");

    await proCta.click();
    await expect(page).toHaveURL(/\/auth\/signup\?plan=pro$/);

    const persisted = await page.evaluate(
      (key) => window.localStorage.getItem(key),
      DRAFT_STORAGE_KEY,
    );
    expect(persisted).not.toBeNull();
    expect(persisted).toContain('"selectedPlan":"pro"');
  });

  test("signed-out Free CTA deep-links to /auth/signup?plan=free", async ({
    page,
  }) => {
    await stubUnauthenticated(page);
    await page.goto("/pricing");

    const freeCta = page.getByTestId("pricing-cta-free");
    await expect(freeCta).toHaveText(/Choose Free/);
    await expect(freeCta).toHaveAttribute("href", "/auth/signup?plan=free");
  });

  test("signed-out Enterprise CTA routes to a talk-to-sales surface", async ({
    page,
  }) => {
    await stubUnauthenticated(page);
    await page.goto("/pricing");

    const entCta = page.getByTestId("pricing-cta-enterprise");
    await expect(entCta).toHaveText(/Talk to sales/);
    const href = await entCta.getAttribute("href");
    expect(href).toBeTruthy();
    expect(href!).toMatch(/^mailto:/);
  });

  test("signed-in Pro CTA links to /billing/subscribe?plan=pro", async ({
    page,
  }) => {
    await stubAuthenticated(page);
    await page.goto("/pricing");

    const proCta = page.getByTestId("pricing-cta-pro");
    await expect(proCta).toHaveAttribute("href", "/billing/subscribe?plan=pro");
  });

  test("?featured=pro override visually expands the Pro tier card", async ({
    page,
  }) => {
    await stubUnauthenticated(page);
    await page.goto("/pricing?featured=pro");

    const proCard = page.locator('[data-tier="pro"]');
    await expect(proCard).toHaveClass(/featured/);
  });
});

async function stubUnauthenticated(page: Page): Promise<void> {
  await page.route("**/api/auth/session", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(null),
    }),
  );
}

async function stubAuthenticated(page: Page): Promise<void> {
  // Minimal NextAuth session shape — the pricing page only inspects
  // `status === "authenticated"`, so the JWT body doesn't need to
  // decode to anything useful.
  await page.route("**/api/auth/session", async (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        user: { email: "buyer@skycot.test" },
        gatewayToken: "header.payload.sig",
        expires: "2099-01-01T00:00:00.000Z",
      }),
    }),
  );
}
