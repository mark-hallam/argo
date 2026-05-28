import { expect, test } from "@playwright/test";

// Regression guard for fix-settings-sub-pages-title-suffix-consistency.
//
// Source: e2e-ux-audit docs/skycot/ux-audits/2026-05-26-2007/
// whole-site-mvp-claims-coverage.md F14 [P2].
//
// Before this brief, four settings sub-routes (`/settings/members`,
// `/settings/api-keys`, `/settings/security`, `/settings/data-export`)
// emitted page-only browser-tab titles — "Members", "API keys",
// "Security", "Data export" — while the other seven sub-routes
// emitted "Settings · SKYCOT". The fix moves the title template
// up to `web/app/(app)/settings/layout.tsx` so every sub-route
// reads "X · Settings · SKYCOT". This spec walks all 11 sub-tabs
// plus the `/settings` root and asserts the contract.
//
// Spec lives at `web/tests/e2e/` (not `web/playwright/settings/`
// as the brief proposed) because `playwright.config.ts` sets
// `testDir: "./tests/e2e"`.

const SETTINGS_TABS = [
  "workspace",
  "members",
  "providers",
  "connectors",
  "knowledge",
  "notify-channels",
  "inbound-channels",
  "notifications",
  "security",
  "api-keys",
  "billing",
  "data-export",
] as const;

const TAB_LABEL: Record<(typeof SETTINGS_TABS)[number], string> = {
  workspace: "Workspace",
  members: "Members",
  providers: "Providers",
  connectors: "Connectors",
  knowledge: "Knowledge",
  "notify-channels": "Notify channels",
  "inbound-channels": "Inbound channels",
  notifications: "Notifications",
  security: "Security",
  "api-keys": "API keys",
  billing: "Billing",
  "data-export": "Data export",
};

test.describe("/settings/<tab> pages have ` · Settings · SKYCOT` title suffix", () => {
  test("/settings root emits `Settings · SKYCOT`", async ({ page }) => {
    const response = await page.goto("/settings", { waitUntil: "domcontentloaded" });
    expect(response, "expected a navigation response for /settings").not.toBeNull();
    const title = await page.title();
    expect(title, `'/settings' title: ${title}`).toBe("Settings · SKYCOT");
  });

  for (const tab of SETTINGS_TABS) {
    test(`/settings/${tab} <title> ends in '${TAB_LABEL[tab]} · Settings · SKYCOT'`, async ({
      page,
    }) => {
      const response = await page.goto(`/settings/${tab}`, {
        waitUntil: "domcontentloaded",
      });
      expect(response, `expected a navigation response for /settings/${tab}`).not.toBeNull();

      const title = await page.title();
      const expected = `${TAB_LABEL[tab]} · Settings · SKYCOT`;
      expect(title, `'/settings/${tab}' title: ${title}`).toBe(expected);

      // No bare brand-only or page-only fallbacks.
      expect(title, `title should end in ' · SKYCOT': ${title}`).toMatch(/ · SKYCOT$/);
      expect(title, `title should not be page-only: ${title}`).not.toBe(TAB_LABEL[tab]);
    });
  }
});
