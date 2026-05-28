import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Regression guard for fix-vertical-landing-cta-and-step-copy-worker-to-teammate
// (UX-AUDIT-T0015 F10 [P2] + F25 [P3]). The operator-locked
// vocabulary memory (project_vocabulary_duty_vs_schedule) is the
// load-bearing rule: in any SPA / hire wizard / marketing / docs /
// settings copy, customer-facing language is "teammate" / "schedule".
// Code/SQL/audit/brief filenames keep the legacy primitive name.
//
// This test enforces the rule on the customer-marketing perimeter
// only — /workers route + internal SPA + API routes are out of scope.

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..", "..");
const WEB_APP = join(REPO_ROOT, "web", "app");

const PERIMETER_DIRS = [
  "landing",
  "for",
  "auth",
  "customers",
  "security",
  "changelog",
  "compare",
  "pricing",
  "about",
];

// Matches "worker" / "workers" / "Worker" / "Workers" as a whole word.
const WORKER_RE = /\b[Ww]orkers?\b/;

function walk(dir: string): string[] {
  let out: string[] = [];
  let entries: string[];
  try {
    entries = readdirSync(dir);
  } catch {
    return out;
  }
  for (const entry of entries) {
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) {
      out = out.concat(walk(full));
    } else if (/\.(ts|tsx|mdx?)$/.test(entry) && !/\.test\.(ts|tsx)$/.test(entry)) {
      out.push(full);
    }
  }
  return out;
}

describe("customer-marketing perimeter uses 'teammate' not 'worker'", () => {
  for (const sub of PERIMETER_DIRS) {
    const dir = join(WEB_APP, sub);
    const files = walk(dir);
    for (const file of files) {
      const rel = relative(REPO_ROOT, file);
      it(`${rel} has no customer-visible 'worker' copy`, () => {
        const source = readFileSync(file, "utf8");
        const offenders: string[] = [];
        for (const rawLine of source.split("\n")) {
          const line = rawLine.trim();
          if (line.startsWith("//") || line.startsWith("/*") || line.startsWith("*")) continue;
          if (WORKER_RE.test(rawLine)) offenders.push(rawLine);
        }
        expect(
          offenders,
          `${rel} contains customer-visible 'worker' copy. Operator-locked vocabulary: customer-facing surfaces use "teammate". Offending lines:\n${offenders.join("\n")}`,
        ).toEqual([]);
      });
    }
  }
});
