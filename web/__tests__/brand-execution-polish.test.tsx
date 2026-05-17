import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..");
const SCAN_DIRS = [
  join(REPO_ROOT, "web", "components"),
  join(REPO_ROOT, "marketing", "components"),
];
const PRIMITIVES = [
  "web/components/ui/Button.tsx",
  "web/components/ui/Card.tsx",
  "web/components/ui/AlertDialog.tsx",
  "web/components/ui/NavDrawer.tsx",
  "web/components/ui/Toast.tsx",
  "web/components/ui/Breadcrumb.tsx",
  "web/components/ui/ProgressDots.tsx",
];

function walk(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      out.push(...walk(full));
    } else if (full.endsWith(".tsx") || full.endsWith(".ts")) {
      out.push(full);
    }
  }
  return out;
}

const ALL_FILES = SCAN_DIRS.flatMap(walk);

describe("brand-execution polish (closes audit P1-2)", () => {
  it("no hardcoded ease-out/ease-in/ease-linear in component files", () => {
    const offenders: string[] = [];
    const re = /ease-(out|in|linear)\b/;
    for (const file of ALL_FILES) {
      const text = readFileSync(file, "utf8");
      const lines = text.split("\n");
      lines.forEach((line, i) => {
        if (re.test(line)) offenders.push(`${file}:${i + 1} → ${line.trim()}`);
      });
    }
    expect(offenders, `Use ease-[var(--motion-ease)] per DESIGN_TOKENS.md motion contract`).toEqual([]);
  });

  it("no off-grid 1.5 spacing utilities (use 1 or 2)", () => {
    const offenders: string[] = [];
    const re = /(gap|p|m|px|py|mx|my)-1\.5\b/;
    for (const file of ALL_FILES) {
      const text = readFileSync(file, "utf8");
      const lines = text.split("\n");
      lines.forEach((line, i) => {
        if (re.test(line)) offenders.push(`${file}:${i + 1} → ${line.trim()}`);
      });
    }
    expect(offenders, `Snap to the 8pt grid: use gap-1 (4px) or gap-2 (8px), not 1.5 (6px)`).toEqual([]);
  });

  it("SPA UI primitives carry at least one sm:/md: responsive variant", () => {
    const missing: string[] = [];
    const re = /\b(sm|md|lg|xl):/;
    for (const rel of PRIMITIVES) {
      const text = readFileSync(join(REPO_ROOT, rel), "utf8");
      if (!re.test(text)) missing.push(rel);
    }
    expect(missing, `Each primitive should adapt below sm: breakpoint`).toEqual([]);
  });
});
