import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Pin: /pricing derives every number from the canonical pricing source of
// truth (docs/skycot/pricing/v1-tiers.toml, DEC-035 / DEC-053). Two guards:
//   1. The committed tiers.data.json matches a fresh parse of the .toml, so
//      a stale checkout (toml edited, JSON not regenerated) fails in CI.
//   2. The pricing JSX has no hardcoded prices / allowances — every number
//      flows through @/lib/pricing/*.
import { buildTiersData } from "../scripts/gen-pricing-tiers.mjs";

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const WEB_ROOT = join(THIS_DIR, "..");
const REPO_ROOT = join(WEB_ROOT, "..");
const TOML_PATH = join(REPO_ROOT, "docs", "skycot", "pricing", "v1-tiers.toml");
const JSON_PATH = join(WEB_ROOT, "lib", "pricing", "tiers.data.json");

describe("pricing tiers source of truth (DEC-035 / DEC-053)", () => {
  it("committed tiers.data.json matches a fresh parse of v1-tiers.toml", () => {
    const toml = readFileSync(TOML_PATH, "utf8");
    const fresh = buildTiersData(toml);
    const committed = JSON.parse(readFileSync(JSON_PATH, "utf8"));
    expect(committed).toEqual(fresh);
  });

  it("Option D tunables are present and margin-safe in the parsed Pro tier", () => {
    const toml = readFileSync(TOML_PATH, "utf8");
    const { pro } = buildTiersData(toml);
    expect(pro.price_monthly_usd).toBeGreaterThan(0);
    expect(pro.included_actions).toBeGreaterThan(0);
    expect(pro.overage_multiplier).toBeGreaterThan(1); // margin-bearing
    expect(pro.default_monthly_cap_usd).toBeGreaterThan(0);
  });

  it("pricing JSX has no hardcoded prices or allowances (single source)", () => {
    const sources = [
      readFileSync(join(WEB_ROOT, "app", "pricing", "page.tsx"), "utf8"),
      readFileSync(join(WEB_ROOT, "lib", "pricing", "pricing-content.ts"), "utf8"),
    ].join("\n");
    // A "$<digits>" literal would be a number baked into copy instead of
    // derived from the toml. "Custom" is the only allowed price label.
    const dollarLiterals = sources.match(/\$\d[\d,]*/g) ?? [];
    expect(
      dollarLiterals,
      `Found hardcoded price literals in pricing JSX: ${dollarLiterals.join(", ")}. ` +
        `Derive from @/lib/pricing/tiers.ts instead.`,
    ).toEqual([]);
    // Action-count literals (e.g. "5,000 actions", "100 actions") must also
    // come from the toml, not be typed into copy.
    const actionLiterals = sources.match(/\b\d[\d,]*\s+actions\b/gi) ?? [];
    expect(
      actionLiterals,
      `Found hardcoded action-count literals: ${actionLiterals.join(", ")}.`,
    ).toEqual([]);
  });
});
