import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import {
  SAMPLE_WORKFLOW_SUMMARIES,
  type WorkflowSummary,
} from "@/lib/sample-workflows";

/**
 * Pins for the /workflows brand-loud catalog. Closes UX-audit T0739
 * F10 (author chip rebalance), F11 (risk pill brand tokens), and F13
 * (TRY-chip single-word stems → >0 cards each).
 */

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const WORKFLOWS_DIR = join(THIS_DIR, "..");
const REPO_ROOT = join(WORKFLOWS_DIR, "..", "..", "..", "..");
const PAGE_TSX = join(WORKFLOWS_DIR, "page.tsx");
const GLOBALS_CSS = join(REPO_ROOT, "web", "app", "globals.css");

function matchesChip(wf: WorkflowSummary, chip: string): boolean {
  const q = chip.toLowerCase();
  return (
    wf.name.toLowerCase().includes(q) ||
    wf.vertical.toLowerCase().includes(q) ||
    wf.description.toLowerCase().includes(q)
  );
}

describe("/workflows catalog — author chips, risk tokens, TRY-chip filters", () => {
  it("no persona is credited as lastEditor on more than a fair share of the seeded cards", () => {
    const counts = new Map<string, number>();
    for (const wf of SAMPLE_WORKFLOW_SUMMARIES) {
      counts.set(wf.lastEditor, (counts.get(wf.lastEditor) ?? 0) + 1);
    }
    // Fair-share ceiling derived from seed size + canonical roster
    // (5: Mariah, Peta, Priya, Jamal, Nori). Pin balance without
    // freezing the seed size — adding substrate-distinctive fixtures
    // grows the catalog but each persona's share stays bounded.
    const CANONICAL_ROSTER_SIZE = 5;
    const fairShareMax = Math.ceil(SAMPLE_WORKFLOW_SUMMARIES.length / CANONICAL_ROSTER_SIZE);
    const offenders = [...counts.entries()].filter(([, n]) => n > fairShareMax);
    expect(
      offenders,
      `author-chip imbalance (cap ${fairShareMax}): ${JSON.stringify([...counts])}`,
    ).toEqual([]);
  });

  it("Peta is credited at least once (Project & Ops persona footprint)", () => {
    const editors = SAMPLE_WORKFLOW_SUMMARIES.map((wf) => wf.lastEditor);
    expect(editors).toContain("Peta");
  });

  it("RISK_COLOR map references CSS variables, never raw HEX literals", () => {
    const src = readFileSync(PAGE_TSX, "utf8");
    const block = src.match(/const RISK_COLOR[^=]*=\s*\{[^}]+\}/);
    expect(block, "RISK_COLOR map not found in page.tsx").not.toBeNull();
    expect(block![0]).toMatch(/var\(--risk-approval-required\)/);
    expect(block![0]).toMatch(/var\(--risk-autonomous\)/);
    expect(block![0]).not.toMatch(/#[0-9a-f]{3,8}\b/i);
  });

  it("globals.css defines the new risk-pill tokens with the legacy amber/teal defaults", () => {
    const css = readFileSync(GLOBALS_CSS, "utf8");
    expect(css).toMatch(/--risk-approval-required:\s*#f59e0b/);
    expect(css).toMatch(/--risk-autonomous:\s*#14b8a6/);
  });

  it("every TRY_CHIP single-word stem resolves to ≥1 card against the current seeds", () => {
    const src = readFileSync(PAGE_TSX, "utf8");
    const block = src.match(/const TRY_CHIPS\s*=\s*\[([^\]]+)\]/);
    expect(block, "TRY_CHIPS not found in page.tsx").not.toBeNull();
    const chips = [...block![1].matchAll(/"([^"]+)"/g)].map((m) => m[1]);
    expect(chips.length).toBeGreaterThanOrEqual(4);
    for (const chip of chips) {
      const hits = SAMPLE_WORKFLOW_SUMMARIES.filter((wf) => matchesChip(wf, chip));
      expect(
        hits.length,
        `TRY chip "${chip}" returned 0 cards — match against name/vertical/description`,
      ).toBeGreaterThan(0);
    }
  });

  it("does not advertise an inflated template count from the live workflow list", () => {
    const src = readFileSync(PAGE_TSX, "utf8");
    expect(src).toMatch(/Open template library/);
    expect(src).toMatch(/open the template library/);
    expect(src).not.toMatch(/TEMPLATE_COUNT\}\+|50\+|53\+/);
    expect(src).not.toContain("TEMPLATE_COUNT");
  });
});
