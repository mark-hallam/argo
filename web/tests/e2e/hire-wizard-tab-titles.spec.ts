import { expect, test } from "@playwright/test";

// Regression pin for fix-hire-page-title-double-suffix.
//
// Source: e2e-ux-audit docs/skycot/ux-audits/2026-05-26-2007/
// whole-site-mvp-claims-coverage.md F17 [P2].
//
// Before the fix, `web/app/(hire)/layout.tsx` declared
// `title: "Hire — SKYCOT"`. The root layout's title.template
// `"%s · SKYCOT"` then composed `"Hire — SKYCOT · SKYCOT"` for /hire
// (and any step page that fell through to the group layout). The fix
// drops the in-string brand so the template adds it exactly once.
//
// The step / done sub-routes already declare their own page-specific
// titles ("Hire · step N", "Welcome to <workerId>") and produce a
// clean single suffix; this spec pins both /hire and /hire/step/N so
// either source of regression is caught.

const ROUTES: ReadonlyArray<{ path: string; expectedTitle: string }> = [
  { path: "/hire", expectedTitle: "Hire · SKYCOT" },
  { path: "/hire/step/2", expectedTitle: "Hire · step 2 · SKYCOT" },
  { path: "/hire/step/3", expectedTitle: "Hire · step 3 · SKYCOT" },
  { path: "/hire/step/4", expectedTitle: "Hire · step 4 · SKYCOT" },
  { path: "/hire/step/5", expectedTitle: "Hire · step 5 · SKYCOT" },
];

test.describe("/hire wizard tab titles have single ` · SKYCOT` suffix", () => {
  for (const { path, expectedTitle } of ROUTES) {
    test(`${path} SSR <title> is "${expectedTitle}"`, async ({ request }) => {
      // request.get bypasses client-side hydration so we read the
      // SSR'd metadata directly. Step 3/4/5 client-side useEffects
      // bounce missing-precondition users back to earlier steps; that
      // post-hydration redirect is intentional and not what this spec
      // is guarding.
      const response = await request.get(path);
      expect(response.status(), `expected HTTP 200 for ${path}`).toBe(200);
      const html = await response.text();
      const match = html.match(/<title>([^<]*)<\/title>/);
      expect(match, `expected a <title> tag in ${path} SSR response`).not.toBeNull();
      const title = match![1];

      expect(title, `title for ${path} should be "${expectedTitle}": got "${title}"`).toBe(
        expectedTitle,
      );

      // Belt-and-braces: catch any double-suffix regression even if the
      // expected-title literal above drifts.
      expect(title, `title should end in ' · SKYCOT': ${title}`).toMatch(/ · SKYCOT$/);
      const suffixCount = (title.match(/ · SKYCOT/g) ?? []).length;
      expect(suffixCount, `title should have exactly one ' · SKYCOT' token: ${title}`).toBe(1);
      expect(title, `title should not contain legacy ' — SKYCOT' brand suffix: ${title}`).not.toMatch(
        / — SKYCOT/,
      );
    });
  }
});
