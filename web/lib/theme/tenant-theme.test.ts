import { describe, expect, it } from "vitest";
import { synthesizeTenantCss } from "./synthesize";
import {
  SKYCOT_DEFAULT_OVERRIDE,
  TENANT_THEME_ID,
  tenantOverrideCss,
  type TenantThemeOverride,
} from "./tenant-theme";

const OVERRIDE: TenantThemeOverride = {
  accent_hex: "#1FAA59",
  neutral_tint: "warm",
  radius: "rounded",
  is_override: true,
};

describe("tenantOverrideCss", () => {
  it("feeds the fetched override straight into synthesizeTenantCss (reuse, not reimplement)", () => {
    const css = tenantOverrideCss(OVERRIDE);
    // Byte-for-byte equal to the canonical synthesizer's output for the
    // mapped input — proving this module does NOT carry a second CSS
    // generator, it delegates to synthesizeTenantCss.
    const expected = synthesizeTenantCss({
      id: TENANT_THEME_ID,
      accentHex: OVERRIDE.accent_hex,
      neutralTint: OVERRIDE.neutral_tint,
      radius: OVERRIDE.radius,
    });
    expect(css).toBe(expected);
  });

  it("produces data-theme-scoped blocks under the tenant id", () => {
    const css = tenantOverrideCss(OVERRIDE);
    expect(css).toContain('[data-theme="tenant-light"]');
    expect(css).toContain('[data-theme="tenant-dark"]');
    // It must NOT touch the locked SKYCOT brand token blocks.
    expect(css).not.toContain('[data-theme="skycot-light"]');
    expect(css).toContain("--primary:");
  });

  it("returns empty CSS when there is no override, leaving the brand untouched", () => {
    expect(tenantOverrideCss(SKYCOT_DEFAULT_OVERRIDE)).toBe("");
    expect(tenantOverrideCss({ ...OVERRIDE, is_override: false })).toBe("");
  });

  it("renders different accents to different CSS", () => {
    const a = tenantOverrideCss({ ...OVERRIDE, accent_hex: "#EC4899" });
    const b = tenantOverrideCss({ ...OVERRIDE, accent_hex: "#5B6BFF" });
    expect(a).not.toBe(b);
  });
});

describe("SKYCOT_DEFAULT_OVERRIDE", () => {
  it("is the locked purple accent and is not an override", () => {
    expect(SKYCOT_DEFAULT_OVERRIDE.accent_hex).toBe("#8B5CF6");
    expect(SKYCOT_DEFAULT_OVERRIDE.is_override).toBe(false);
  });
});
