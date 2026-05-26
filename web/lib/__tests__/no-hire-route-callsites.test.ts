import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

/**
 * Regression guard for fix-in-product-hire-ctas-rewire-to-onboarding-canonical.
 *
 * Every in-product "Hire" CTA used to point at `/hire` (the parallel
 * wizard at `web/app/(hire)/hire/...`) while the canonical demo entry is
 * `/onboarding/first-hire`. We rewired all 7 product-surface callsites;
 * this test fails if a new `href="/hire"` or `router.push("/hire")`
 * sneaks back in under `web/app/` or `web/components/`.
 *
 * The wizard's own internal navigation (under `web/app/(hire)/`) is
 * excluded — those router.push calls move between wizard steps and are
 * not "in-product CTAs".
 */

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const WEB_DIR = join(THIS_DIR, "..", "..");
const APP_DIR = join(WEB_DIR, "app");
const COMPONENTS_DIR = join(WEB_DIR, "components");

// The parallel wizard lives here. Its internal router.push("/hire") /
// `/hire/step/N` calls are wizard-internal navigation, not in-product
// CTAs, so they are out of scope for this guard.
const HIRE_WIZARD_DIR = join(APP_DIR, "(hire)");
const HIRE_COMPONENTS_DIR = join(COMPONENTS_DIR, "skycot", "hire");
const HIRE_SHELL_FILE = join(COMPONENTS_DIR, "skycot", "HireShell.tsx");

const FORBIDDEN_PATTERNS: Array<{ pattern: RegExp; description: string }> = [
  {
    pattern: /href=["']\/hire["']/,
    description: 'href="/hire"',
  },
  {
    pattern: /router\.push\(["']\/hire["']\)/,
    description: 'router.push("/hire")',
  },
];

function walk(dir: string, acc: string[]): string[] {
  let entries: string[];
  try {
    entries = readdirSync(dir);
  } catch {
    return acc;
  }
  for (const name of entries) {
    const full = join(dir, name);
    let st;
    try {
      st = statSync(full);
    } catch {
      continue;
    }
    if (st.isDirectory()) {
      if (full === HIRE_WIZARD_DIR) continue;
      if (full === HIRE_COMPONENTS_DIR) continue;
      if (name === "node_modules" || name === ".next") continue;
      walk(full, acc);
    } else if (/\.(tsx?|jsx?)$/.test(name)) {
      if (full === HIRE_SHELL_FILE) continue;
      acc.push(full);
    }
  }
  return acc;
}

describe("no in-product callsite points at /hire (parallel wizard)", () => {
  it("scans web/app and web/components for forbidden href/router.push patterns", () => {
    const files = [...walk(APP_DIR, []), ...walk(COMPONENTS_DIR, [])];
    const offenders: Array<{ file: string; line: number; match: string; rule: string }> = [];

    for (const file of files) {
      const text = readFileSync(file, "utf8");
      const lines = text.split("\n");
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i] ?? "";
        for (const { pattern, description } of FORBIDDEN_PATTERNS) {
          if (pattern.test(line)) {
            offenders.push({
              file: relative(WEB_DIR, file),
              line: i + 1,
              match: line.trim(),
              rule: description,
            });
          }
        }
      }
    }

    expect(
      offenders,
      `Found callsites pointing at the parallel /hire wizard. ` +
        `Use /onboarding/first-hire (or /genesis) instead. ` +
        `See docs/build-plan/streams/fix-in-product-hire-ctas-rewire-to-onboarding-canonical.md.`,
    ).toEqual([]);
  });
});
