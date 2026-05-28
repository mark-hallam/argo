import { expect, test } from "@playwright/test";

// Regression pin for fix-onboarding-step-pages-tab-title-via-layout-metadata.
//
// Source: e2e-ux-audit docs/skycot/ux-audits/2026-05-26-2007/
// whole-site-mvp-claims-coverage.md F1 [P1] / F9 [P1].
//
// Before the fix all five onboarding step pages were `"use client"`
// components without `metadata` exports. Next.js disallows metadata on
// client components, so every step fell through to the root layout's
// default title — the demo Scene 3 recording showed the same
// `SKYCOT — Hire your first AI teammate` tab on every step.
//
// The fix adds a server-component `layout.tsx` per step with a unique
// `metadata.title`. The global template `"%s · SKYCOT"` composes the
// brand suffix once, so the rendered tab is e.g. `Welcome · Onboarding · SKYCOT`.
//
// This spec walks all five steps and asserts each tab title is the
// expected per-step string and that none of them fall back to the
// site default. If any step regresses to `"use client"` without a
// sibling `layout.tsx`, this spec fails.

const STEPS: ReadonlyArray<{ path: string; title: string }> = [
  { path: "/onboarding/welcome", title: "Welcome · Onboarding · SKYCOT" },
  { path: "/onboarding/workspace", title: "Workspace · Onboarding · SKYCOT" },
  {
    path: "/onboarding/connect-provider",
    title: "Connect provider · Onboarding · SKYCOT",
  },
  { path: "/onboarding/pick-vertical", title: "Pick vertical · Onboarding · SKYCOT" },
  { path: "/onboarding/first-hire", title: "First hire · Onboarding · SKYCOT" },
];

test.describe("each onboarding step has a unique per-step tab title", () => {
  for (const { path, title } of STEPS) {
    test(`${path} → ${title}`, async ({ page }) => {
      await page.goto(path, { waitUntil: "domcontentloaded" });
      await expect(page).toHaveTitle(title);
    });
  }

  test("no step falls back to the site-default tab title", async ({ page }) => {
    for (const { path } of STEPS) {
      await page.goto(path, { waitUntil: "domcontentloaded" });
      const title = await page.title();
      expect(
        title,
        `step ${path} regressed to the site-default tab title`,
      ).not.toMatch(/^SKYCOT — Hire your first AI teammate/);
    }
  });
});
