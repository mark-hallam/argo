import { expect, test } from "@playwright/test";

// Regression guard for fix-docs-sub-routes-all-404.
//
// Source: e2e-ux-audit docs/skycot/ux-audits/2026-05-26-2152/
// whole-site-mvp-claims-coverage.md F1 [P0].
//
// Before this brief, the /docs index advertised 10 sub-route cards
// (getting-started, integrations, skills, workflows, api, audit, plus
// four featured-guide deep links) — every one of which returned HTTP
// 404 because the directory tree under web/app/docs/ contained only
// page.tsx. This spec walks all 10 routes, asserts HTTP 200, and
// asserts each renders a recognisable <h1> heading so a future
// refactor that removes the stubs fails CI instead of silently
// re-breaking the docs drill chain.

const SUB_ROUTES: ReadonlyArray<readonly [string, RegExp]> = [
  ["/docs/getting-started", /getting started/i],
  ["/docs/getting-started/hire-first-teammate", /hire your first ai teammate/i],
  ["/docs/integrations", /integrations/i],
  ["/docs/integrations/slack-approvals", /slack approval gates/i],
  ["/docs/skills", /skills/i],
  ["/docs/workflows", /workflows/i],
  ["/docs/workflows/visual-canvas", /workflow builder/i],
  ["/docs/api", /api reference/i],
  ["/docs/audit", /audit/i],
  ["/docs/audit/trust-ratchet", /trust ratchet/i],
];

test.describe("/docs sub-routes resolve with stub content", () => {
  for (const [route, h1Match] of SUB_ROUTES) {
    test(`${route} returns 200 and renders an <h1>`, async ({ page }) => {
      const response = await page.goto(route, { waitUntil: "domcontentloaded" });
      expect(response, `expected a navigation response for ${route}`).not.toBeNull();
      expect(response!.status(), `expected HTTP 200 for ${route}`).toBe(200);

      const heading = page.locator("h1").first();
      await expect(heading).toBeVisible();
      await expect(heading).toHaveText(h1Match);

      // Stub pages must surface the "request this article" CTA pointing
      // at hello@skycot.com so visitors have a way to nudge SKYCOT to
      // prioritise the full write-up.
      const requestCta = page.getByRole("link", { name: /request this article/i });
      await expect(requestCta).toBeVisible();
      await expect(requestCta).toHaveAttribute("href", /^mailto:hello@skycot\.com/);

      // Breadcrumb back to /docs is the navigation contract the
      // docs index assumes.
      const backLink = page.getByRole("link", { name: /back to all docs/i });
      await expect(backLink).toBeVisible();
      await expect(backLink).toHaveAttribute("href", "/docs");
    });
  }
});
