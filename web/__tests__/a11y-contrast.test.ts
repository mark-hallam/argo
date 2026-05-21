import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..");
const GLOBALS_CSS = join(REPO_ROOT, "web", "app", "globals.css");
const SCAN_ROOT = join(REPO_ROOT, "web");
const SCAN_SKIP = new Set(["node_modules", ".next", ".turbo", "__tests__", "tests"]);

function walk(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    if (SCAN_SKIP.has(entry)) continue;
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      out.push(...walk(full));
    } else if (/\.(tsx?|css|s?css)$/.test(full)) {
      out.push(full);
    }
  }
  return out;
}

function relLuminance(r: number, g: number, b: number): number {
  const channel = (c: number) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrastRatio(rgbA: [number, number, number], rgbB: [number, number, number]): number {
  const la = relLuminance(...rgbA);
  const lb = relLuminance(...rgbB);
  const [light, dark] = la > lb ? [la, lb] : [lb, la];
  return (light + 0.05) / (dark + 0.05);
}

function hexToRgb(hex: string): [number, number, number] {
  const clean = hex.replace("#", "");
  const n = parseInt(clean, 16);
  return [(n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff];
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

describe("a11y color contrast (WCAG 2.1 AA, 1.4.3)", () => {
  it("--muted-foreground meets 4.5:1 against --background in light theme", () => {
    const css = readFileSync(GLOBALS_CSS, "utf8");
    const lightBlock = css.match(/:root,\s*\[data-theme="skycot-light"\]\s*\{([\s\S]+?)\}/);
    expect(lightBlock, "skycot-light theme block missing from globals.css").not.toBeNull();
    const block = lightBlock![1];

    const fgMatch = block.match(/--muted-foreground:\s*oklch\(([^)]+)\)/);
    expect(fgMatch, "--muted-foreground oklch token missing").not.toBeNull();
    const [L, C, h] = fgMatch![1].trim().split(/\s+/).map(Number);
    const fgRgb = oklchToRgb(L, C, h);

    const bgRgb = hexToRgb("#fafaff");
    const ratio = contrastRatio(fgRgb, bgRgb);
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it("--muted-foreground meets 4.5:1 against --card (white surface)", () => {
    const css = readFileSync(GLOBALS_CSS, "utf8");
    const lightBlock = css.match(/:root,\s*\[data-theme="skycot-light"\]\s*\{([\s\S]+?)\}/);
    const fgMatch = lightBlock![1].match(/--muted-foreground:\s*oklch\(([^)]+)\)/);
    const [L, C, h] = fgMatch![1].trim().split(/\s+/).map(Number);
    const mutedRgb = oklchToRgb(L, C, h);
    const cardRgb: [number, number, number] = [255, 255, 255]; // --card is oklch(1.00 0.000 0)
    const ratio = contrastRatio(mutedRgb, cardRgb);
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it("dark-theme --muted-foreground meets 4.5:1 against dark --background", () => {
    const css = readFileSync(GLOBALS_CSS, "utf8");
    const darkBlock = css.match(/\.dark,\s*\[data-theme="skycot-dark"\]\s*\{([\s\S]+?)\}/);
    expect(darkBlock).not.toBeNull();
    const block = darkBlock![1];
    const bgMatch = block.match(/--background:\s*oklch\(([^)]+)\)/);
    const fgMatch = block.match(/--muted-foreground:\s*oklch\(([^)]+)\)/);
    expect(bgMatch).not.toBeNull();
    expect(fgMatch).not.toBeNull();
    const [bL, bC, bh] = bgMatch![1].trim().split(/\s+/).map(Number);
    const [fL, fC, fh] = fgMatch![1].trim().split(/\s+/).map(Number);
    const ratio = contrastRatio(oklchToRgb(fL, fC, fh), oklchToRgb(bL, bC, bh));
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it("no hardcoded #71717a remains in web/ (regression guard — fails AA against #fafaff)", () => {
    const offenders: string[] = [];
    const re = /#71717[aA]\b/;
    for (const file of walk(SCAN_ROOT)) {
      const text = readFileSync(file, "utf8");
      const lines = text.split("\n");
      lines.forEach((line, i) => {
        if (re.test(line)) offenders.push(`${file}:${i + 1} → ${line.trim()}`);
      });
    }
    expect(
      offenders,
      "Hex #71717a (~4.4:1 on #fafaff) fails WCAG AA. Use #52525b (slate-600, 7.0:1) or var(--muted-foreground).",
    ).toEqual([]);
  });
});
