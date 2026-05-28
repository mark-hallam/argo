import { expect, test, type BrowserContext, type Page } from "@playwright/test";

/**
 * `/auth/*` layouts force the light theme via a MutationObserver
 * (`app/auth/layout.tsx`) that fights `next-themes` whenever the
 * resolved theme is dark. On a host whose OS reports
 * `prefers-color-scheme: dark`, the resulting class-toggle ping-pong
 * can peg the page's JS thread for the lifetime of the page. Combine
 * that with `page.setViewportSize` (which retriggers media-query
 * listeners) and the multi-viewport version of this test would
 * deadlock on resize — `boundingBox` / `evaluate` would timeout
 * because the main thread never yielded.
 *
 * Two structural changes make this deterministic:
 *  1. `localStorage.theme = "light"` is pinned via an init script so
 *     `next-themes` resolves to light from the first render, matching
 *     the design intent for auth surfaces.
 *  2. Each viewport gets its own fresh `BrowserContext`-owned page
 *     — no in-place resize — so we never re-fire the listener cascade
 *     against an already-hydrated page.
 */

const VIEWPORTS = [
  { name: "desktop", width: 1600, height: 900, expectedStageWidth: 720 },
  { name: "tablet", width: 768, height: 900, expectedStageWidth: 720 },
  { name: "mobile", width: 375, height: 812, expectedStageWidth: 375 },
] as const;

async function openSignup(
  context: BrowserContext,
  viewport: { width: number; height: number },
): Promise<Page> {
  const page = await context.newPage();
  await page.emulateMedia({ colorScheme: "light" });
  await page.setViewportSize(viewport);
  await page.goto("/auth/signup");
  return page;
}

test.describe("/auth/signup editorial centering", () => {
  // Each test below opens a fresh `/auth/signup` page and hits the dev
  // server's turbopack pipeline; running them in parallel under one
  // dev server makes `page.evaluate` flaky on cold compile. Running
  // serially is fast enough (each test is < 5s warm) and removes the
  // flake.
  test.describe.configure({ mode: "serial" });

  test.beforeEach(async ({ context }) => {
    await context.addInitScript(() => {
      try {
        localStorage.setItem("theme", "light");
      } catch {
        // ignore
      }
    });
  });

  for (const vp of VIEWPORTS) {
    test(`keeps the signup stage centered at ${vp.name} width`, async ({ context }) => {
      const page = await openSignup(context, { width: vp.width, height: vp.height });
      // Read every dimension we need in a single `evaluate` round-trip.
      // Repeated `evaluate` calls against this page can deadlock when
      // the auth-layout theme observer re-fires (see file header), so
      // collect once instead of querying piece-by-piece. Hero copy
      // was unified with /landing + /onboarding/welcome via #2548
      // (drop "AI" from "Hire your first AI teammate"); the H1 text
      // assertion in the desktop run pins the new copy.
      const measurements = await page.evaluate(() => {
        const stageEl = document.querySelector(
          '[data-testid="auth-stage"]',
        ) as HTMLElement | null;
        const formEl = document.querySelector(
          '[data-testid="signup-form"]',
        ) as HTMLElement | null;
        const heroEl = document.querySelector("h1.hero") as HTMLElement | null;
        if (!stageEl) return null;
        const stage = stageEl.getBoundingClientRect();
        const form = formEl ? formEl.getBoundingClientRect() : null;
        return {
          stage: { x: stage.x, width: stage.width },
          formWidth: form ? form.width : null,
          centerX: document.documentElement.clientWidth / 2,
          heroText: heroEl ? heroEl.textContent : null,
        };
      });
      expect(measurements).not.toBeNull();
      expect(Math.round(measurements!.stage.width)).toBe(vp.expectedStageWidth);
      expect(
        Math.abs(
          measurements!.stage.x + measurements!.stage.width / 2 - measurements!.centerX,
        ),
      ).toBeLessThanOrEqual(1);

      if (vp.name === "desktop") {
        expect(measurements!.formWidth).not.toBeNull();
        expect(Math.round(measurements!.formWidth!)).toBeLessThanOrEqual(460);
        // H1 splits "Hire your first" + <br> + "teammate."; textContent
        // concatenates without whitespace.
        expect(measurements!.heroText).toMatch(/hire your first\s*teammate/i);
      }
    });
  }
});
