import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Regression pin for fix-marketing-cta-teammate-vs-worker-vocab-drift
// (ux-e2e-audit F5 [P1] + F7 [P2]). The load-bearing brand promise —
// MISSION Claim 1 "Teammate, not tool" — gets undercut when marketing
// CTAs say "Hire your first worker" / "Hire a worker". "Worker" is the
// in-product runtime/SQL primitive (nav row, crates/argo-runtime); it
// must NOT appear as a verb-object on outward-facing marketing pages.
//
// Scope (mirrors the brief's grep target):
//   web/app/{landing,for,pricing,compare,customers,about,contact,
//             security,changelog,marketing}/**/page.tsx + co-located
//             _components/*.tsx
//
// Allowed exceptions:
//   - "Hire {Name}" — the in-wizard commit button (handled in /hire,
//     not in marketing dirs, so out-of-scope by directory anyway).
//   - The capitalized noun "Workers" used as a nav-row / runtime
//     entity label is fine; the test only flags the verb pattern.

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..");
const APP_ROOT = join(REPO_ROOT, "web", "app");

const MARKETING_DIRS = [
  "landing",
  "for",
  "pricing",
  "compare",
  "customers",
  "about",
  "contact",
  "security",
  "changelog",
  "marketing",
];

const BANNED_PATTERNS: Array<{ label: string; rx: RegExp }> = [
  { label: '"Hire your first worker"', rx: /Hire\s+your\s+first\s+workers?\b/i },
  { label: '"Hire a worker"', rx: /Hire\s+(?:a|an|the)\s+workers?\b/i },
];

function walk(dir: string): string[] {
  const out: string[] = [];
  let entries: string[];
  try {
    entries = readdirSync(dir);
  } catch {
    return out;
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
      out.push(...walk(full));
    } else if (st.isFile() && /\.(tsx|ts)$/.test(name)) {
      out.push(full);
    }
  }
  return out;
}

function collectMarketingSources(): string[] {
  const files: string[] = [];
  for (const dir of MARKETING_DIRS) {
    files.push(...walk(join(APP_ROOT, dir)));
  }
  return files.filter((p) => !/\.test\.(ts|tsx)$/.test(p));
}

describe("marketing CTA vocab — 'teammate' not 'worker'", () => {
  const sources = collectMarketingSources();

  it("discovers marketing sources to scan (sanity)", () => {
    expect(sources.length).toBeGreaterThan(0);
  });

  for (const banned of BANNED_PATTERNS) {
    it(`no marketing page contains ${banned.label}`, () => {
      const hits: string[] = [];
      for (const file of sources) {
        const text = readFileSync(file, "utf8");
        if (banned.rx.test(text)) {
          hits.push(file.replace(REPO_ROOT + "/", ""));
        }
      }
      expect(
        hits,
        `Marketing surfaces must say "teammate", not "worker", in CTA copy. ` +
          `Found ${banned.label} in: ${hits.join(", ")}. ` +
          `Per MISSION Claim 1 — "Teammate, not tool" — the verb-object pattern ` +
          `must read "teammate" on marketing pages. ("Workers" as a runtime/nav ` +
          `noun is fine and stays untouched.)`,
      ).toEqual([]);
    });
  }

  it("/landing renders its waitlist-hero primary CTA; /for vertical landings keep 'Hire your first teammate'", () => {
    // GTM pivot: /landing's hero primary CTA is now the governance-walkthrough
    // + early-access waitlist hero (the old "Hire your first teammate" CTA moved
    // off the landing hero). The "teammate, not worker" guarantee still holds —
    // see the banned-pattern guards above — but the landing primary CTA copy
    // changed, so pin the current copy here rather than the retired phrase.
    const landing = readFileSync(join(APP_ROOT, "landing", "page.tsx"), "utf8");
    expect(landing).toMatch(/Book a governance walkthrough/);

    const verticalShell = readFileSync(
      join(APP_ROOT, "for", "_components", "VerticalLandingShell.tsx"),
      "utf8",
    );
    expect(verticalShell).toMatch(/Hire your first teammate/);
  });
});
