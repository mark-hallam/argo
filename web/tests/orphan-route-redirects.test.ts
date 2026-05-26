import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Regression guard for fix-spa-polish-help-retro-billing-orphan-pages.
//
// Source: e2e-ux-audit
// docs/skycot/ux-audits/2026-05-26-1441/whole-site-mvp-claims-coverage.md
// F08 + F18.
//
// Three orphan-route patterns were noted:
//
//   1. `/help` — a "we're building" placeholder. Replaced with a
//      server-side redirect to `/docs` so existing inbound links
//      (WizardEditorialShell + auth shells' "Help" anchor, /sla
//      footer, /auth/signin body) land on a useful surface.
//
//   2. `/retro` — a real tenant-wide retro index that is
//      intentionally hidden from the primary sidebar nav (retros
//      are accessed per-worker from `/workers/<id>`). Documented
//      inline.
//
//   3. `/billing` — the canonical billing surface. The audit
//      flagged it as an "orphan duplicate" of `/settings/billing`,
//      but `/settings/billing/page.tsx` links *to* `/billing` for
//      the real plan/checkout flow. Premise corrected and the role
//      documented inline so the next audit doesn't re-flag it.
//
// This test pins the three guarantees above so future refactors
// don't silently regress them.

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..");
const WEB_DIR = join(REPO_ROOT, "web");

const HELP_PAGE = join(WEB_DIR, "app", "help", "page.tsx");
const RETRO_PAGE = join(WEB_DIR, "app", "(app)", "retro", "page.tsx");
const BILLING_PAGE = join(WEB_DIR, "app", "(app)", "billing", "page.tsx");
const SITEMAP = join(WEB_DIR, "app", "sitemap.ts");

describe("orphan-route polish (F08 + F18)", () => {
  it("/help redirects to /docs via next/navigation", () => {
    const source = readFileSync(HELP_PAGE, "utf8");
    expect(
      source,
      "/help page must import `redirect` from next/navigation",
    ).toMatch(/import\s*\{\s*redirect\s*\}\s*from\s*["']next\/navigation["']/);
    expect(
      source,
      '/help page must call `redirect("/docs")`',
    ).toMatch(/redirect\(\s*["']\/docs["']\s*\)/);
  });

  it("/retro is documented as intentionally-hidden", () => {
    const source = readFileSync(RETRO_PAGE, "utf8");
    // JSDoc comments wrap across lines with a leading ` * ` prefix.
    // Strip those prefixes + collapse whitespace before matching so
    // the assertion isn't sensitive to how the comment is wrapped.
    const collapsed = source.replace(/^\s*\*\s?/gm, "").replace(/\s+/g, " ");
    expect(
      collapsed,
      "/retro page must carry a top-of-file comment documenting why it is not in sidebar nav",
    ).toMatch(/intentionally hidden from the primary sidebar nav/i);
  });

  it("/billing is documented as the canonical billing surface", () => {
    const source = readFileSync(BILLING_PAGE, "utf8");
    const collapsed = source.replace(/^\s*\*\s?/gm, "").replace(/\s+/g, " ");
    expect(
      collapsed,
      "/billing page must carry a top-of-file comment naming it the canonical billing surface",
    ).toMatch(/Canonical billing surface/i);
  });

  it("sitemap.ts does not list /help, /retro, or /billing", () => {
    const source = readFileSync(SITEMAP, "utf8");
    for (const path of ["/help", "/retro", "/billing"]) {
      // The sitemap uses `${base}<path>` template literals, so any
      // occurrence of the bare path string in url-position would
      // be a sitemap entry. Match it via a substring check that
      // tolerates the template-literal context.
      expect(
        source,
        `sitemap.ts must not list ${path}`,
      ).not.toContain(`}${path}\``);
    }
  });
});
