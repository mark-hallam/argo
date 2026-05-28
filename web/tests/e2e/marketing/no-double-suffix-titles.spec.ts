import { expect, test } from "@playwright/test";

// Regression guard for fix-marketing-for-vertical-pages-double-suffix-title.
//
// Source: e2e-ux-audit docs/skycot/ux-audits/2026-05-26-2007/
// whole-site-mvp-claims-coverage.md F6 [P1].
//
// Before this brief, every `/for/<vertical>` page emitted a doubled
// brand suffix in `<title>` (e.g. "AI sales teammates that never
// sleep — SKYCOT · SKYCOT"): per-vertical metadata set its own
// " — SKYCOT" suffix AND inherited the global title.template
// "%s · SKYCOT" from app/layout.tsx. This spec walks all 6 verticals
// and asserts a single " · SKYCOT" suffix with no leading " — SKYCOT"
// brand collision.

const VERTICALS = ["sales", "recruiting", "marketing", "finance", "ops", "support"] as const;

test.describe("/for/<vertical> pages have single ` · SKYCOT` title suffix", () => {
  for (const v of VERTICALS) {
    test(`/for/${v} <title> has exactly one ` + "` · SKYCOT` suffix", async ({ page }) => {
      const response = await page.goto(`/for/${v}`, { waitUntil: "domcontentloaded" });
      expect(response, `expected a navigation response for /for/${v}`).not.toBeNull();
      expect(response!.status(), `expected HTTP 200 for /for/${v}`).toBe(200);

      const title = await page.title();

      // Single " · SKYCOT" suffix, exactly one middle-dot brand token.
      expect(title, `title should end in ' · SKYCOT': ${title}`).toMatch(/ · SKYCOT$/);
      const middleDotCount = (title.match(/ · SKYCOT/g) ?? []).length;
      expect(middleDotCount, `title should have exactly one ' · SKYCOT' token: ${title}`).toBe(1);

      // No legacy " — SKYCOT" em-dash brand suffix anywhere in the title.
      expect(title, `title should not contain legacy ' — SKYCOT' brand suffix: ${title}`).not.toMatch(/ — SKYCOT/);
    });
  }
});
