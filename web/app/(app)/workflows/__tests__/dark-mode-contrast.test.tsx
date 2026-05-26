import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

/**
 * Dark-mode contrast contract for the /workflows surface.
 *
 * The 2026-05-26 UX audit (ux-e2e-audit-T2309MEL F4/F22/F23/F25) caught
 * the workflow cards rendering `background: rgb(255,255,255)` literal
 * white against `--foreground` off-white in dark mode (white-on-white
 * "first_listing", "qualify_lead", TRY pills, and 3-of-4 toolbar
 * buttons). Root cause: hardcoded `background: white;` in the three
 * `page.css` files instead of consuming the `--card` token the theme
 * provider toggles via `[data-theme="skycot-dark"]` / `.dark`.
 *
 * This spec is a static contract — vitest runs with `css: false` so we
 * can't measure computed styles. Instead we:
 *   1. assert no `background: white;` (or inline `background: "white"`)
 *      remains in the in-scope page sources, and
 *   2. compute the WCAG 2.1 AA contrast between the dark-mode `--card`
 *      and `--foreground` tokens from globals.css and assert ≥ 4.5:1
 *      so a future drift in the theme tokens themselves still fails.
 */

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const WORKFLOWS_DIR = join(THIS_DIR, "..");
const REPO_ROOT = join(WORKFLOWS_DIR, "..", "..", "..", "..");
const GLOBALS_CSS = join(REPO_ROOT, "web", "app", "globals.css");

const IN_SCOPE_SOURCES = [
  join(WORKFLOWS_DIR, "page.css"),
  join(WORKFLOWS_DIR, "page.tsx"),
  join(WORKFLOWS_DIR, "templates", "page.css"),
  join(WORKFLOWS_DIR, "templates", "page.tsx"),
  join(WORKFLOWS_DIR, "[verticalId]", "[name]", "page.css"),
  join(WORKFLOWS_DIR, "[verticalId]", "[name]", "page.tsx"),
];

function relLuminance(r: number, g: number, b: number): number {
  const channel = (c: number) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrastRatio(a: [number, number, number], b: [number, number, number]): number {
  const la = relLuminance(...a);
  const lb = relLuminance(...b);
  const [light, dark] = la > lb ? [la, lb] : [lb, la];
  return (light + 0.05) / (dark + 0.05);
}

function oklchToRgb(L: number, C: number, hDeg: number): [number, number, number] {
  const h = (hDeg * Math.PI) / 180;
  const a = C * Math.cos(h);
  const b = C * Math.sin(h);
  const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
  const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
  const s_ = L - 0.0894841775 * a - 1.291485548 * b;
  const l = l_ ** 3;
  const m = m_ ** 3;
  const s = s_ ** 3;
  const r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
  const g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
  const bl = -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s;
  const toSrgb = (c: number) => {
    const clamped = Math.max(0, Math.min(1, c));
    const v = clamped <= 0.0031308
      ? 12.92 * clamped
      : 1.055 * Math.pow(clamped, 1 / 2.4) - 0.055;
    return Math.round(v * 255);
  };
  return [toSrgb(r), toSrgb(g), toSrgb(bl)];
}

function darkTokenRgb(css: string, name: string): [number, number, number] {
  const darkBlock = css.match(/\.dark,\s*\[data-theme="skycot-dark"\]\s*\{([\s\S]+?)\n\}/);
  expect(darkBlock, "dark theme block missing from globals.css").not.toBeNull();
  const re = new RegExp(`--${name}:\\s*oklch\\(([^)]+)\\)`);
  const match = darkBlock![1].match(re);
  expect(match, `--${name} oklch token missing from dark theme`).not.toBeNull();
  const [L, C, h] = match![1].trim().split(/\s+/).map(Number);
  return oklchToRgb(L, C, h);
}

describe("/workflows dark-mode contrast", () => {
  it("no hardcoded `background: white` remains in workflows page sources", () => {
    const offenders: string[] = [];
    const cssLiteral = /background:\s*white\b/;
    const tsxLiteral = /background:\s*["']white["']/;
    for (const file of IN_SCOPE_SOURCES) {
      const text = readFileSync(file, "utf8");
      text.split("\n").forEach((line, i) => {
        if (cssLiteral.test(line) || tsxLiteral.test(line)) {
          offenders.push(`${file}:${i + 1} → ${line.trim()}`);
        }
      });
    }
    expect(
      offenders,
      "Hardcoded `background: white` in /workflows surface — use `var(--card, #ffffff)` so the dark theme can override it.",
    ).toEqual([]);
  });

  it("workflow card class uses var(--card) so the dark theme overrides it", () => {
    const css = readFileSync(join(WORKFLOWS_DIR, "page.css"), "utf8");
    expect(css).toMatch(/\.workflow-card\s*\{[^}]*background:\s*var\(--card/);
  });

  it("template card class uses var(--card)", () => {
    const css = readFileSync(join(WORKFLOWS_DIR, "templates", "page.css"), "utf8");
    expect(css).toMatch(/\.template-card\s*\{[^}]*background:\s*var\(--card/);
  });

  it("workflow detail toolbar button uses var(--card) (3-of-4 secondary CTAs)", () => {
    const css = readFileSync(
      join(WORKFLOWS_DIR, "[verticalId]", "[name]", "page.css"),
      "utf8",
    );
    expect(css).toMatch(/\.wfd-btn\s*\{[^}]*background:\s*var\(--card/);
  });

  it("dark --card vs dark --foreground meets WCAG AA 4.5:1", () => {
    const css = readFileSync(GLOBALS_CSS, "utf8");
    const card = darkTokenRgb(css, "card");
    const fg = darkTokenRgb(css, "foreground");
    const ratio = contrastRatio(card, fg);
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });
});
