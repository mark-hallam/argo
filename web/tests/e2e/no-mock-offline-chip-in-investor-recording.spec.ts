import { expect, test } from "@playwright/test";

/**
 * Investor-recording regression pin — F7 of the
 * 2026-05-26-2007 chrome-live-walk-and-playwright UX audit.
 *
 * The defect: when the gateway 404'd a not-yet-wired root-listing
 * endpoint, the SPA fell through to the bundled `SAMPLE_*` fixtures
 * with `source = "mock-offline"` and rendered a visible chip in the
 * page heading. That chip ("MOCK-OFFLINE", "MOCK (OFFLINE)", or
 * "MOCK-BYPASS") read to an investor as "broken software" — fine for
 * developer diagnostics, fatal during the v1.0 demo MP4 recording.
 *
 * The fix moved chip rendering behind an opt-in
 * `NEXT_PUBLIC_SKYCOT_SHOW_DATA_SOURCE_CHIP=1` flag. The Playwright
 * webServer (`playwright.config.ts`) does NOT set that var, so this
 * spec exercises the recording-day default: no visible "MOCK-OFFLINE"
 * / "MOCK-BYPASS" pill on any of the heading-bearing pages an investor
 * would walk in the demo runbook.
 *
 * The underlying source stays inspectable via a `data-source`
 * attribute on the page-header wrapper (asserted on /team and
 * /approvals); DevTools can still confirm which path the page is
 * reading without showing the chip to a user.
 */
const HEADING_PAGES = [
  { path: "/team", name: "team" },
  { path: "/audit", name: "audit" },
  { path: "/", name: "approvals" }, // / redirects through approvals inbox
  { path: "/skills", name: "skills" },
  { path: "/workflows", name: "workflows" },
];

for (const { path, name } of HEADING_PAGES) {
  test(`${name}: no MOCK-OFFLINE chip is visible in the default recording profile`, async ({
    page,
  }) => {
    await page.goto(path);

    // The two chip surfaces this fix gates.
    await expect(page.getByTestId("data-source-chip")).toHaveCount(0);
    await expect(page.getByTestId("team-data-source-chip")).toHaveCount(0);

    // Heuristic: even if a new chip surface gets added later that
    // forgets the opt-in flag, the literal text the camera would see
    // ("MOCK-OFFLINE" or "mock (offline)") must not appear anywhere
    // on the page.
    await expect(page.getByText(/MOCK-OFFLINE/i)).toHaveCount(0);
    await expect(page.getByText(/mock\s*\(\s*offline\s*\)/i)).toHaveCount(0);
  });
}

test("team page-header data-source attribute remains inspectable for DevTools", async ({
  page,
}) => {
  await page.goto("/team");
  const header = page.getByTestId("team-page-header");
  const source = await header.getAttribute("data-source");
  expect(source).not.toBeNull();
  // Under the Playwright env profile (auth-bypass on, gateway port
  // closed), the SPA should be on one of the mock fall-throughs.
  expect(["mock-offline", "mock-bypass", "loading", "error", "gateway"]).toContain(source);
});

test("approvals page-header data-source attribute remains inspectable for DevTools", async ({
  page,
}) => {
  await page.goto("/");
  const header = page.getByTestId("approvals-page-header");
  const source = await header.getAttribute("data-source");
  expect(source).not.toBeNull();
  expect(["mock-offline", "mock-auth-bypass", "live", "unknown"]).toContain(source);
});
