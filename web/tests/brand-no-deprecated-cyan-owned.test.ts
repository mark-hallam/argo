import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const OWNED_ROOTS = ["app", "components", "lib"] as const;
const SOURCE_EXTENSIONS = new Set([".css", ".ts", ".tsx"]);
const EXCLUDED_SEGMENTS = new Set([
  "_dev",
  "genesis",
  "pitch",
  "gtm",
  "marketing",
]);
const EXCLUDED_FILES = new Set([
  "lib/api/genesis.ts",
  "lib/api/pitch.ts",
  "lib/api/gtm.ts",
  "lib/api/marketing.ts",
]);

const DEPRECATED_CYAN_HEX = "#0891" + "b2";
const DEPRECATED_PURPLE_HEX = "#8b5c" + "f6";

const FORBIDDEN = [
  { label: "deprecated cyan hex", pattern: new RegExp(DEPRECATED_CYAN_HEX, "i") },
  { label: "deprecated cyan rgba", pattern: /rgba\(\s*8\s*,\s*145\s*,\s*178/i },
  {
    label: "deprecated purple-cyan gradient",
    pattern: new RegExp(
      `${DEPRECATED_PURPLE_HEX}[\\s\\S]{0,120}${DEPRECATED_CYAN_HEX}|${DEPRECATED_CYAN_HEX}[\\s\\S]{0,120}${DEPRECATED_PURPLE_HEX}`,
      "i",
    ),
  },
  { label: "deprecated cyan custom property", pattern: /--(?:ob-)?cyan\b/i },
] as const;

function isExcluded(relativePath: string): boolean {
  if (EXCLUDED_FILES.has(relativePath)) return true;
  return relativePath.split(path.sep).some((segment) => EXCLUDED_SEGMENTS.has(segment));
}

function collectSourceFiles(root: string): string[] {
  const out: string[] = [];
  const stack = [root];

  while (stack.length > 0) {
    const current = stack.pop()!;
    const entries = fs.readdirSync(current, { withFileTypes: true });
    for (const entry of entries) {
      const absolute = path.join(current, entry.name);
      const relative = path.relative(process.cwd(), absolute);
      if (isExcluded(relative)) continue;
      if (entry.isDirectory()) {
        stack.push(absolute);
        continue;
      }
      if (SOURCE_EXTENSIONS.has(path.extname(entry.name))) out.push(absolute);
    }
  }

  return out.sort();
}

describe("owned frontend brand debt", () => {
  it("does not reintroduce deprecated cyan or purple-cyan gradients", () => {
    const files = OWNED_ROOTS.flatMap((root) => collectSourceFiles(path.join(process.cwd(), root)));
    const failures: string[] = [];

    for (const file of files) {
      const source = fs.readFileSync(file, "utf8");
      for (const rule of FORBIDDEN) {
        if (rule.pattern.test(source)) {
          failures.push(`${path.relative(process.cwd(), file)}: ${rule.label}`);
        }
      }
    }

    expect(failures).toEqual([]);
  });
});
