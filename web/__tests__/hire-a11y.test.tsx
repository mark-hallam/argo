import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..");
const SHELL_FILE = join(
  REPO_ROOT,
  "web",
  "components",
  "auth",
  "WizardEditorialShell.tsx",
);

function relLuminance(r: number, g: number, b: number): number {
  const channel = (c: number) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrastRatio(
  rgbA: [number, number, number],
  rgbB: [number, number, number],
): number {
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

function readTokenInWizEditorial(token: string): string {
  const css = readFileSync(SHELL_FILE, "utf8");
  // Extract the .wiz-editorial block contents.
  const block = css.match(/\.wiz-editorial\s*\{([\s\S]*?)\n\}/);
  expect(block, ".wiz-editorial block missing from WizardEditorialShell").not.toBeNull();
  // Match the LAST declaration of the token in the block (CSS cascade).
  const re = new RegExp(`--${token}:\\s*(#[0-9a-fA-F]{6})\\s*;`, "g");
  const matches = Array.from(block![1].matchAll(re));
  expect(
    matches.length,
    `--${token} hex value missing from .wiz-editorial`,
  ).toBeGreaterThan(0);
  return matches[matches.length - 1][1];
}

/**
 * Regression guard for ux-audit-T2309MEL F9: `/hire` step 1 label was
 * rendering white-on-white when the user had dark theme active because
 * `WizardEditorialShell` hardcodes a light page bg but child steps use
 * shadcn `text-[var(--foreground)]` utilities that flip with the root
 * theme. Fix locks the semantic color tokens to light values within
 * the wizard scope; this test pins those values and their contrast
 * against the locked bg.
 */
describe("hire wizard a11y — locked WizardEditorialShell color tokens", () => {
  it("locks --foreground to dark-ink (>=4.5:1 against the wizard's light bg)", () => {
    const foreground = readTokenInWizEditorial("foreground");
    const background = readTokenInWizEditorial("background");
    const ratio = contrastRatio(hexToRgb(foreground), hexToRgb(background));
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it("locks --muted-foreground to a slate-600-equivalent passing 4.5:1", () => {
    const muted = readTokenInWizEditorial("muted-foreground");
    const background = readTokenInWizEditorial("background");
    const ratio = contrastRatio(hexToRgb(muted), hexToRgb(background));
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it("pins --background to the warm-near-white F1 page color", () => {
    expect(readTokenInWizEditorial("background").toLowerCase()).toBe("#fafaff");
  });
});
