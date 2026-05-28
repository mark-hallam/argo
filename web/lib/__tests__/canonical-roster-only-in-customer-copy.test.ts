import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Regression guard for fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira
// (UX-AUDIT-T0015 F7 [P1]). The customer-marketing perimeter must
// never name a worker who does not exist in the product. The
// canonical demo roster is Mariah / Peta / Priya / Jamal / Nori
// (with Patrick as the second Project & Ops worker). Names that
// previously leaked: Skylar (sales), Fen (finance), Mira
// (marketing), plus Rio from the earlier /workflows wave.

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..", "..");
const WEB_APP = join(REPO_ROOT, "web", "app");

const PERIMETER_DIRS = [
  "for",
  "landing",
  "customers",
  "compare",
  "pricing",
  "auth",
  "(setup)/onboarding",
];

const FORBIDDEN = ["Skylar", "Fen", "Mira", "Rio"] as const;

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
    } else if (/\.(ts|tsx|mdx?|json)$/.test(entry)) {
      out.push(full);
    }
  }
  return out;
}

describe("customer-marketing perimeter only names canonical workers", () => {
  for (const sub of PERIMETER_DIRS) {
    const dir = join(WEB_APP, sub);
    const files = walk(dir);
    for (const file of files) {
      const rel = file.replace(REPO_ROOT + "/", "");
      it(`${rel} names no forbidden non-canonical worker`, () => {
        const source = readFileSync(file, "utf8");
        for (const name of FORBIDDEN) {
          const pattern = new RegExp(`\\b${name}\\b`);
          expect(
            pattern.test(source),
            `${rel} contains forbidden non-canonical worker name "${name}". The canonical demo roster is Mariah / Peta / Priya / Jamal / Nori (with Patrick as the second Project & Ops worker).`,
          ).toBe(false);
        }
      });
    }
  }
});
