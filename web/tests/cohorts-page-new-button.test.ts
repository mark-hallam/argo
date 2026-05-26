import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Closes whole-site-mvp-claims-coverage F19 [P2] — `/cohorts/new` was
// only reachable from the empty-state CTA, so operators with existing
// cohorts could not author a new one without leaving the page. The
// header now always renders a "+ New cohort" link to `/cohorts/new`,
// regardless of cohort count. Static source check keeps this fast
// (no need to mount the page with its query-client + tab state).

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const COHORTS_PAGE = join(
  THIS_DIR,
  "..",
  "app",
  "(app)",
  "cohorts",
  "page.tsx",
);

describe("/cohorts page header CTA", () => {
  const source = readFileSync(COHORTS_PAGE, "utf8");

  it("renders a + New cohort link in the page header", () => {
    expect(source).toMatch(/data-testid="cohorts-header-new-cta"/);
    expect(source).toMatch(/\+ New cohort/);
  });

  it("points the header CTA at /cohorts/new", () => {
    const match = source.match(
      /<Link[\s\S]*?data-testid="cohorts-header-new-cta"[\s\S]*?<\/Link>/,
    );
    expect(match, "header CTA <Link> block should exist").not.toBeNull();
    expect(match?.[0]).toMatch(/href="\/cohorts\/new"/);
  });

  it("keeps the empty-state CTA wired to /cohorts/new", () => {
    const match = source.match(
      /<Link[\s\S]*?data-testid="cohorts-create-cta"[\s\S]*?<\/Link>/,
    );
    expect(match, "empty-state CTA <Link> block should exist").not.toBeNull();
    expect(match?.[0]).toMatch(/href="\/cohorts\/new"/);
  });
});
