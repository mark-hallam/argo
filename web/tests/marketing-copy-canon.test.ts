import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  SKYCOT_NAME,
  SKYCOT_TAGLINE,
  SKYCOT_TITLE_DEFAULT,
  SKYCOT_TITLE_TAGLINE_FIRST,
} from "@/lib/marketing-copy";

// Regression guard for fix-tagline-copy-duplicated-three-locations
// (e2e-ux-audit F22 [P3]). Locks two invariants:
//   1. The canonical constants resolve to the operator-approved strings.
//   2. The three metadata-bearing layouts reference the constants
//      rather than the literal tagline, so a future reword is a
//      single-line change in lib/marketing-copy.ts.

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..");

const CANON_LAYOUTS = [
  join(REPO_ROOT, "web", "app", "layout.tsx"),
  join(REPO_ROOT, "web", "app", "landing", "layout.tsx"),
  join(REPO_ROOT, "web", "app", "auth", "signup", "layout.tsx"),
];

describe("marketing-copy canonical constants", () => {
  it("preserves the operator-approved tagline + name", () => {
    expect(SKYCOT_NAME).toBe("SKYCOT");
    expect(SKYCOT_TAGLINE).toBe("Hire your first AI teammate");
    expect(SKYCOT_TITLE_DEFAULT).toBe("SKYCOT — Hire your first AI teammate");
    expect(SKYCOT_TITLE_TAGLINE_FIRST).toBe(
      "Hire your first AI teammate — SKYCOT",
    );
  });

  for (const layout of CANON_LAYOUTS) {
    it(`${layout.replace(REPO_ROOT + "/", "")} imports from marketing-copy rather than inlining the tagline`, () => {
      const source = readFileSync(layout, "utf8");
      expect(
        source,
        `Expected ${layout} to import the canonical tagline from @/lib/marketing-copy instead of inlining "${SKYCOT_TAGLINE}".`,
      ).toMatch(/from\s+["']@\/lib\/marketing-copy["']/);
      expect(
        source.includes(`"${SKYCOT_TAGLINE}"`),
        `Expected ${layout} to not contain the literal tagline string "${SKYCOT_TAGLINE}". Reference SKYCOT_TAGLINE (or a composite) from @/lib/marketing-copy instead.`,
      ).toBe(false);
    });
  }
});
