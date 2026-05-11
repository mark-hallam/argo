/**
 * Synthesize a tenant theme from a single brand accent + a few
 * preferences. Used by the Operator console's "configure theme"
 * surface in Track E.
 *
 * We work in OKLCH because it is perceptually uniform — picking
 * lightness offsets in OKLCH produces palettes that LOOK like
 * scales, regardless of hue. RGB / HSL palettes go grey or muddy
 * at the ends of the scale; OKLCH does not.
 *
 * Output is a CSS string snippet you can inject into a <style>
 * tag or persist server-side to ship to the client.
 */

export interface TenantThemeInput {
  /** Stable tenant id; becomes data-theme="<id>-light" / "<id>-dark". */
  id: string;
  /** Hex accent color, e.g. "#5B6BFF". Converted to OKLCH internally. */
  accentHex: string;
  /** Cool / neutral / warm bias of the surrounding neutrals. */
  neutralTint?: "cool" | "neutral" | "warm";
  /** Radius scale family. */
  radius?: "sharp" | "default" | "rounded";
}

interface OKLCH {
  l: number;
  c: number;
  h: number;
}

/** Cheap hex → OKLCH. Sufficient for theme synthesis; not color-managed. */
function hexToOklch(hex: string): OKLCH {
  const m = hex.replace("#", "").match(/.{1,2}/g);
  if (!m || m.length < 3) return { l: 0.55, c: 0.18, h: 264 };
  const [r, g, b] = m.slice(0, 3).map((s) => parseInt(s, 16) / 255);
  // Convert sRGB → linear RGB → OKLab → OKLCH (Björn Ottosson)
  const toLinear = (v: number) => (v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4));
  const lr = toLinear(r);
  const lg = toLinear(g);
  const lb = toLinear(b);
  const l = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb);
  const m2 = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb);
  const s = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb);
  const lab_l = 0.2104542553 * l + 0.793617785 * m2 - 0.0040720468 * s;
  const lab_a = 1.9779984951 * l - 2.428592205 * m2 + 0.4505937099 * s;
  const lab_b = 0.0259040371 * l + 0.7827717662 * m2 - 0.808675766 * s;
  const c = Math.sqrt(lab_a * lab_a + lab_b * lab_b);
  let h = (Math.atan2(lab_b, lab_a) * 180) / Math.PI;
  if (h < 0) h += 360;
  return { l: lab_l, c, h };
}

function oklch({ l, c, h }: OKLCH): string {
  return `oklch(${l.toFixed(3)} ${c.toFixed(3)} ${h.toFixed(1)})`;
}

const NEUTRAL_HUE: Record<NonNullable<TenantThemeInput["neutralTint"]>, number> = {
  cool: 248,
  neutral: 0,
  warm: 50,
};

const RADIUS_SCALE: Record<NonNullable<TenantThemeInput["radius"]>, [number, number, number]> = {
  sharp: [2, 3, 6],
  default: [4, 6, 12],
  rounded: [6, 10, 18],
};

/**
 * Produce a CSS snippet defining `[data-theme="<id>-light"]` and
 * `[data-theme="<id>-dark"]` blocks. Mount via <style> in the
 * <head> of a tenant-scoped layout, or serve as a stylesheet from
 * the gateway.
 */
export function synthesizeTenantCss(input: TenantThemeInput): string {
  const accent = hexToOklch(input.accentHex);
  const tint = input.neutralTint ?? "cool";
  const neutralHue = NEUTRAL_HUE[tint];
  const neutralChroma = tint === "neutral" ? 0 : 0.015;
  const [rSm, rMd, rLg] = RADIUS_SCALE[input.radius ?? "default"];

  const lightPrimary = { l: 0.55, c: Math.min(0.22, accent.c), h: accent.h };
  const darkPrimary = { l: 0.72, c: Math.min(0.18, accent.c * 0.85), h: accent.h };

  return `
[data-theme="${input.id}-light"] {
  --background:            ${oklch({ l: 0.99, c: 0.005, h: neutralHue })};
  --foreground:            ${oklch({ l: 0.18, c: 0.020, h: neutralHue })};
  --card:                  oklch(1.000 0 0);
  --card-foreground:       ${oklch({ l: 0.18, c: 0.020, h: neutralHue })};
  --popover:               oklch(1.000 0 0);
  --popover-foreground:    ${oklch({ l: 0.18, c: 0.020, h: neutralHue })};
  --primary:               ${oklch(lightPrimary)};
  --primary-foreground:    ${oklch({ l: 0.99, c: 0.005, h: accent.h })};
  --secondary:             ${oklch({ l: 0.96, c: neutralChroma, h: neutralHue })};
  --secondary-foreground:  ${oklch({ l: 0.25, c: 0.020, h: neutralHue })};
  --muted:                 ${oklch({ l: 0.96, c: neutralChroma, h: neutralHue })};
  --muted-foreground:      ${oklch({ l: 0.52, c: 0.015, h: neutralHue })};
  --accent:                ${oklch({ l: 0.96, c: neutralChroma, h: neutralHue })};
  --accent-foreground:     ${oklch({ l: 0.25, c: 0.020, h: neutralHue })};
  --border:                ${oklch({ l: 0.92, c: neutralChroma, h: neutralHue })};
  --input:                 ${oklch({ l: 0.92, c: neutralChroma, h: neutralHue })};
  --ring:                  ${oklch(lightPrimary)};
  --radius-sm-value:       ${rSm}px;
  --radius-md-value:       ${rMd}px;
  --radius-lg-value:       ${rLg}px;
}

[data-theme="${input.id}-dark"] {
  --background:            ${oklch({ l: 0.16, c: neutralChroma, h: neutralHue })};
  --foreground:            ${oklch({ l: 0.96, c: 0.010, h: neutralHue })};
  --card:                  ${oklch({ l: 0.20, c: neutralChroma, h: neutralHue })};
  --card-foreground:       ${oklch({ l: 0.96, c: 0.010, h: neutralHue })};
  --popover:               ${oklch({ l: 0.20, c: neutralChroma, h: neutralHue })};
  --popover-foreground:    ${oklch({ l: 0.96, c: 0.010, h: neutralHue })};
  --primary:               ${oklch(darkPrimary)};
  --primary-foreground:    ${oklch({ l: 0.16, c: 0.020, h: accent.h })};
  --secondary:             ${oklch({ l: 0.26, c: neutralChroma, h: neutralHue })};
  --secondary-foreground:  ${oklch({ l: 0.96, c: 0.010, h: neutralHue })};
  --muted:                 ${oklch({ l: 0.26, c: neutralChroma, h: neutralHue })};
  --muted-foreground:      ${oklch({ l: 0.65, c: 0.015, h: neutralHue })};
  --accent:                ${oklch({ l: 0.26, c: neutralChroma, h: neutralHue })};
  --accent-foreground:     ${oklch({ l: 0.96, c: 0.010, h: neutralHue })};
  --border:                ${oklch({ l: 0.30, c: neutralChroma, h: neutralHue })};
  --input:                 ${oklch({ l: 0.30, c: neutralChroma, h: neutralHue })};
  --ring:                  ${oklch(darkPrimary)};
  --radius-sm-value:       ${rSm}px;
  --radius-md-value:       ${rMd}px;
  --radius-lg-value:       ${rLg}px;
}
`.trim();
}
