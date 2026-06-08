"use client";

/**
 * Applies a tenant's persisted white-label override at runtime.
 *
 * On mount it fetches `GET /api/v1/tenant/theme` and, when the gateway
 * reports `is_override: true`, (1) injects the synthesized
 * `data-theme`-scoped CSS — produced by reusing `synthesizeTenantCss` via
 * `tenantOverrideCss` — into a single `<style>` in `<head>`, and (2)
 * activates the registered `tenant` theme through the existing
 * `ThemeProvider` context so the `DataThemeBridge` flips `<html
 * data-theme>` to `tenant-light` / `tenant-dark`. The two together make
 * the tenant's accent take effect.
 *
 * ## Brand guardrail (DEC-027)
 *
 * When there is NO override, it removes any injected style and resets the
 * active theme to SKYCOT — so the locked brand tokens always stand for a
 * non-white-label tenant, and a tenant who later clears their override
 * self-heals back to the default.
 *
 * Renders nothing. Must live inside `<ThemeProvider>` (it uses
 * `useTenantTheme`).
 */

import { useEffect } from "react";
import { useTenantTheme } from "@/components/skycot/ThemeProvider";
import { SKYCOT_THEME } from "@/lib/theme/themes";
import {
  TENANT_THEME_ID,
  fetchTenantThemeOverride,
  tenantOverrideCss,
} from "@/lib/theme/tenant-theme";

const STYLE_ELEMENT_ID = "skycot-tenant-theme-override";

export function TenantThemeStyle(): null {
  const { setThemeById } = useTenantTheme();

  useEffect(() => {
    let cancelled = false;

    function clearInjectedStyle() {
      document.getElementById(STYLE_ELEMENT_ID)?.remove();
    }

    fetchTenantThemeOverride()
      .then((override) => {
        if (cancelled) return;
        const css = tenantOverrideCss(override);
        if (!css) {
          // No override (or empty): keep the locked brand. Reset both the
          // injected CSS and the active theme so a cleared override heals.
          clearInjectedStyle();
          setThemeById(SKYCOT_THEME.id);
          return;
        }
        let styleEl = document.getElementById(STYLE_ELEMENT_ID) as HTMLStyleElement | null;
        if (!styleEl) {
          styleEl = document.createElement("style");
          styleEl.id = STYLE_ELEMENT_ID;
          document.head.appendChild(styleEl);
        }
        styleEl.textContent = css;
        setThemeById(TENANT_THEME_ID);
      })
      .catch(() => {
        // A failed fetch must never break the page nor white-label it —
        // leave the locked brand tokens in place.
      });

    return () => {
      cancelled = true;
    };
  }, [setThemeById]);

  return null;
}
