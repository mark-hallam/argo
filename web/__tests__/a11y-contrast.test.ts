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

/**
 * Composite `fg` at `alpha` over opaque `bg` (sRGB linear-ish approximation
 * matching the browser's alpha blend at the byte level). Used to model the
 * StatusPill tint background `bg-[var(--tone)]/10` sitting over the card/page.
 */
function over(
  fg: [number, number, number],
  bg: [number, number, number],
  alpha: number,
): [number, number, number] {
  return [
    Math.round(fg[0] * alpha + bg[0] * (1 - alpha)),
    Math.round(fg[1] * alpha + bg[1] * (1 - alpha)),
    Math.round(fg[2] * alpha + bg[2] * (1 - alpha)),
  ];
}

/** Parse an `oklch(L C h)` token value from a CSS block into an RGB triple. */
function oklchTokenRgb(block: string, token: string): [number, number, number] {
  const m = block.match(new RegExp(`${token}:\\s*oklch\\(([^)]+)\\)`));
  if (!m) throw new Error(`token ${token} (oklch) missing from CSS block`);
  const [L, C, h] = m[1].trim().split(/\s+/).map(Number);
  return oklchToRgb(L, C, h);
}

/** Parse a `#rrggbb` hex token value from a CSS block into an RGB triple. */
function hexTokenRgb(block: string, token: string): [number, number, number] {
  const m = block.match(new RegExp(`${token}:\\s*(#[0-9a-fA-F]{6})\\b`));
  if (!m) throw new Error(`token ${token} (hex) missing from CSS block`);
  return hexToRgb(m[1]);
}

function lightBlockOf(css: string): string {
  const b = css.match(/:root,\s*\[data-theme="skycot-light"\]\s*\{([\s\S]+?)\n\}/);
  if (!b) throw new Error("skycot-light theme block missing from globals.css");
  return b[1];
}

function darkBlockOf(css: string): string {
  const b = css.match(/\.dark,\s*\[data-theme="skycot-dark"\]\s*\{([\s\S]+?)\n\}/);
  if (!b) throw new Error("skycot-dark theme block missing from globals.css");
  return b[1];
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

/* --------------------------------------------------------------------------
   A11Y-1 / A11Y-3 / A11Y-P2-4 — darker on-light TEXT tokens.

   These guard the readability fix: small status labels, the auth error line,
   and the trust-strip caption were measured ~2.3–2.7:1 before. The fix adds
   darker TEXT-ONLY tokens (the saturated fill/tint/dot colours are unchanged).
   Each label TEXT is checked >=4.5:1 against the surface it actually sits on.
   -------------------------------------------------------------------------- */
const PAGE = hexToRgb("#fafaff"); // --brand-bg (auth + most light surfaces)
const WHITE: [number, number, number] = [255, 255, 255]; // --card

// StatusPill tint background = base status colour at /10 alpha. The badge can
// sit on either the white card or the warm page; we assert against BOTH (and
// against plain white, the worst realistic case where the tint is invisible).
// `error` tone draws its tint/border from --destructive, so that's its base.
const TONES: ReadonlyArray<{ text: string; base: string; tone: string }> = [
  { tone: "success", text: "--success-text", base: "--success" },
  { tone: "warning", text: "--warning-text", base: "--warning" },
  { tone: "error", text: "--error-text", base: "--destructive" },
  { tone: "info", text: "--info-text", base: "--info" },
];

describe("a11y status-label TEXT contrast (StatusPill, A11Y-1)", () => {
  it("each light status-text token meets 4.5:1 over its tint bg (white card + warm page)", () => {
    const css = readFileSync(GLOBALS_CSS, "utf8");
    const block = lightBlockOf(css);
    for (const { tone, text, base } of TONES) {
      const textRgb = oklchTokenRgb(block, text);
      const baseRgb = oklchTokenRgb(block, base);
      const tintOverWhite = over(baseRgb, WHITE, 0.1);
      const tintOverPage = over(baseRgb, PAGE, 0.1);
      const rWhite = contrastRatio(textRgb, tintOverWhite);
      const rPage = contrastRatio(textRgb, tintOverPage);
      const rPlainWhite = contrastRatio(textRgb, WHITE);
      expect(rWhite, `${tone}: ${text} vs tint-over-white-card`).toBeGreaterThanOrEqual(4.5);
      expect(rPage, `${tone}: ${text} vs tint-over-warm-page`).toBeGreaterThanOrEqual(4.5);
      expect(rPlainWhite, `${tone}: ${text} vs plain white (tint-invisible worst case)`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("each dark status-text token meets 4.5:1 against the dark --card and --background", () => {
    const css = readFileSync(GLOBALS_CSS, "utf8");
    const block = darkBlockOf(css);
    const cardRgb = oklchTokenRgb(block, "--card");
    const bgRgb = oklchTokenRgb(block, "--background");
    for (const { tone, text } of TONES) {
      const textRgb = oklchTokenRgb(block, text);
      expect(contrastRatio(textRgb, cardRgb), `${tone}: ${text} vs dark --card`).toBeGreaterThanOrEqual(4.5);
      expect(contrastRatio(textRgb, bgRgb), `${tone}: ${text} vs dark --background`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("the saturated tint/dot tokens are UNCHANGED (brand lock — fill colours not repurposed)", () => {
    const css = readFileSync(GLOBALS_CSS, "utf8");
    const block = lightBlockOf(css);
    // These are the dot + tint + border colours and MUST stay as-is.
    expect(block).toMatch(/--success:\s*oklch\(0\.62 0\.16 145\)/);
    expect(block).toMatch(/--warning:\s*oklch\(0\.72 0\.16 75\)/);
    expect(block).toMatch(/--info:\s*oklch\(0\.62 0\.17 230\)/);
    expect(block).toMatch(/--destructive:\s*oklch\(0\.58 0\.21 25\)/);
  });
});

describe("a11y auth funnel TEXT contrast (A11Y-3 / A11Y-P2-4)", () => {
  it("--brand-coral-text meets 4.5:1 on the #fafaff auth page (error text)", () => {
    const css = readFileSync(GLOBALS_CSS, "utf8");
    const block = lightBlockOf(css);
    const coralText = hexTokenRgb(block, "--brand-coral-text");
    expect(contrastRatio(coralText, PAGE)).toBeGreaterThanOrEqual(4.5);
  });

  it("decorative --brand-coral is UNCHANGED (#ff6b6b) — only the text sibling is new", () => {
    const css = readFileSync(GLOBALS_CSS, "utf8");
    const block = lightBlockOf(css);
    expect(block).toMatch(/--brand-coral:\s*#ff6b6b\b/);
  });

  it("--brand-ink-3 (trust-strip caption text) meets 4.5:1 on the #fafaff page", () => {
    const css = readFileSync(GLOBALS_CSS, "utf8");
    const block = lightBlockOf(css);
    const ink3 = hexTokenRgb(block, "--brand-ink-3");
    expect(contrastRatio(ink3, PAGE)).toBeGreaterThanOrEqual(4.5);
  });

  it("--brand-ink-4 hairline token is UNCHANGED (#a1a1aa) — text was rerouted, not the token", () => {
    const css = readFileSync(GLOBALS_CSS, "utf8");
    const block = lightBlockOf(css);
    expect(block).toMatch(/--brand-ink-4:\s*#a1a1aa\b/);
  });
});
