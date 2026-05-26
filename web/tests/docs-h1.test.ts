import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Regression guard for fix-docs-page-missing-h1-heading-hierarchy.
//
// Source: e2e-ux-audit docs/skycot/ux-audits/2026-05-26-1028/
// whole-site-mvp-claims-coverage.md F14 [P1].
//
// The audit reported /docs as having no <h1>, only <h3> card titles —
// an SEO and a11y regression (screen readers can't find the document
// title). On inspection the existing brand-loud editorial hero already
// declares one <h1>; the audit's regex
// (`<h[1234][^>]*>[^<]+</h[1234]>`) didn't match it because the h1
// contains a nested <strong>. This test locks the invariant so any
// future refactor that drops the h1 fails CI instead of silently
// regressing the docs page heading hierarchy.

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..");
const DOCS_PAGE = join(REPO_ROOT, "web", "app", "docs", "page.tsx");

describe("/docs page heading hierarchy", () => {
  it("declares exactly one <h1> element", () => {
    const source = readFileSync(DOCS_PAGE, "utf8");
    const matches = source.match(/<h1\b/g) ?? [];
    expect(
      matches.length,
      `Expected exactly one <h1> in ${DOCS_PAGE}, found ${matches.length}. ` +
        `The /docs hub must source a single page-level heading for SEO and screen-reader navigation.`,
    ).toBe(1);
  });
});
