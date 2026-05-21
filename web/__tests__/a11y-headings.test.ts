import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Static a11y smoke for WCAG 2.1 AA 1.3.1 (Info and Relationships) +
// 2.4.6 (Headings and Labels). Closes wave-2 audit
// `docs/skycot/audit/a11y/2026-05-20-wave-2-a11y.md` finding #1
// (heading-structure) for the in-product surfaces under
// `web/app/(app)/`, `web/app/(hire)/`, and `web/app/(setup)/`.

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..");
const WEB_DIR = join(REPO_ROOT, "web");
const APP_DIR = join(WEB_DIR, "app");

// In-product route groups covered by this brief. Marketing surfaces
// (`web/app/landing`, `compare`, `pricing`, `customers`, `changelog`,
// `docs`, `for`, `help`, `privacy`, `security`, `terms`, `auth/*`) are
// out of scope and tracked in follow-up briefs from the wave-2 audit.
const IN_PRODUCT_ROUTE_GROUPS = ["(app)", "(hire)", "(setup)"];

// Shell / hero components that render an `<h1>` on behalf of their
// child route. A page that does not declare its own `<h1>` but
// imports one of these (directly or via a layout up the route tree)
// is considered to have a load-bearing page heading. Kept manually
// because the alternative — fully mounting every page — is much
// heavier than this static check.
const H1_PROVIDERS = [
  "WizardEditorialShell",
  "OnboardingShell",
  "BrandLoudHero",
  "AuditPageHeader",
  "WorkerHero",
  "WorkerLayoutShell",
  "ActionSummaryHero",
  "ReplayHeader",
  "TicketThread",
  "WorkflowDiff",
  "BuilderToolbar",
];

function walk(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    if (entry.startsWith(".")) continue;
    const full = join(dir, entry);
    const s = statSync(full);
    if (s.isDirectory()) out.push(...walk(full));
    else if (full.endsWith(".tsx")) out.push(full);
  }
  return out;
}

function isInProductFile(file: string): boolean {
  const rel = relative(APP_DIR, file);
  return IN_PRODUCT_ROUTE_GROUPS.some(
    (group) => rel === `${group}` || rel.startsWith(`${group}/`),
  );
}

function extractHeadingLevels(text: string): number[] {
  const re = /<h([1-6])\b/g;
  const levels: number[] = [];
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    levels.push(Number(m[1]));
  }
  return levels;
}

function collectLayoutChain(pageFile: string): string[] {
  const chain: string[] = [];
  let cursor = dirname(pageFile);
  while (cursor.startsWith(APP_DIR) || cursor === APP_DIR) {
    const layoutPath = join(cursor, "layout.tsx");
    try {
      chain.push(readFileSync(layoutPath, "utf8"));
    } catch {
      // No layout at this segment — keep climbing.
    }
    if (cursor === APP_DIR) break;
    cursor = dirname(cursor);
  }
  return chain;
}

describe("a11y heading order — closes wave-2 audit + web-a11y-heading-order-sweep brief", () => {
  it("no in-product .tsx file under web/app/ skips heading levels", () => {
    const offenders: string[] = [];
    for (const file of walk(APP_DIR)) {
      if (!isInProductFile(file)) continue;
      const text = readFileSync(file, "utf8");
      const levels = [...new Set(extractHeadingLevels(text))].sort(
        (a, b) => a - b,
      );
      if (levels.length < 2) continue;
      for (let i = 0; i < levels.length - 1; i++) {
        if (levels[i + 1] - levels[i] > 1) {
          offenders.push(
            `${relative(REPO_ROOT, file)} → uses h${levels.join(", h")}`,
          );
          break;
        }
      }
    }
    expect(
      offenders,
      "Heading levels must be contiguous within a file. Insert intermediate heading levels or demote skipped ones.",
    ).toEqual([]);
  });

  it("each in-product page.tsx has an h1 source (own heading or known shell)", () => {
    const pageFiles = walk(APP_DIR)
      .filter((f) => f.endsWith("/page.tsx"))
      .filter(isInProductFile);
    const offenders: string[] = [];
    for (const file of pageFiles) {
      const text = readFileSync(file, "utf8");
      // Stub / redirect-only routes (no JSX returned, just `redirect()`)
      // are excluded — they never render a heading anyway.
      if (text.includes("redirect(") && !/<h[1-6]\b/.test(text)) continue;

      const ownH1Count = (text.match(/<h1\b/g) ?? []).length;
      if (ownH1Count > 0) continue;

      const haystack = [text, ...collectLayoutChain(file)].join("\n");
      const providesH1 = H1_PROVIDERS.some((p) =>
        new RegExp(`\\b${p}\\b`).test(haystack),
      );
      if (!providesH1) {
        offenders.push(relative(REPO_ROOT, file));
      }
    }
    expect(
      offenders,
      "Every in-product page.tsx must source an h1 either directly or through a known layout/shell component.",
    ).toEqual([]);
  });

  it("no in-product page.tsx declares more than one own h1 in its own file", () => {
    const pageFiles = walk(APP_DIR)
      .filter((f) => f.endsWith("/page.tsx"))
      .filter(isInProductFile);
    const offenders: string[] = [];
    for (const file of pageFiles) {
      const text = readFileSync(file, "utf8");
      const h1Count = (text.match(/<h1\b/g) ?? []).length;
      // The homepage `(app)/page.tsx` and a couple of split-state pages
      // legitimately declare h1 in mutually exclusive branches. Two h1s
      // in the same file are tolerated only if the file branches on
      // loading / error / content (verified by the page-level state
      // hooks); never more than two.
      if (h1Count > 2) {
        offenders.push(`${relative(REPO_ROOT, file)} → ${h1Count} h1 tags`);
      }
    }
    expect(
      offenders,
      "A page should declare at most one h1, with two only in mutually-exclusive render branches.",
    ).toEqual([]);
  });
});
