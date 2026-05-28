"use client";

import { Info, X } from "lucide-react";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import {
  useDataSource,
  type DataSource,
} from "@/lib/skycot/data-source-context";
import {
  isDemoMode,
  recordingModeReadinessWarning,
} from "@/lib/skycot/demo-mode";

const DISMISS_STORAGE_KEY = "skycot:demo-banner-dismissed";
export const HIDE_DEMO_BANNER_ENV_VAR = "NEXT_PUBLIC_HIDE_DEMO_BANNER" as const;

function isBannerSuppressed(): boolean {
  return process.env.NEXT_PUBLIC_HIDE_DEMO_BANNER === "1";
}

/**
 * Copy variant for a given data source. The banner's claim about the
 * underlying data must match what the page is actually showing — a
 * viewer pausing on `/audit` against the live (empty) gateway should
 * NOT see "showing sample data" (UX-AUDIT-T0015 F8). When no fetcher
 * has published yet (initial paint, or the page has no data layer),
 * `source` is `null` and we fall back to the conservative pre-existing
 * "showing sample data" copy so we don't lie in either direction.
 */
function bannerCopy(source: DataSource | null): { lead: string; rest: string } {
  switch (source) {
    case "live":
      return { lead: "Demo mode —", rest: " sign in to see your own workspace." };
    case "mock-offline":
      return {
        lead: "Demo mode —",
        rest: " backend unreachable, showing seed data.",
      };
    case "mock-bypass":
      return {
        lead: "Demo mode —",
        rest:
          " showing curated sample data. Sign in to see your own workspace.",
      };
    case null:
      return {
        lead: "Demo mode —",
        rest: " showing sample data. Sign in to see your own workspace.",
      };
  }
}

/**
 * Slim banner that pins to the top of the app shell whenever
 * `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`. Tells the viewer they're looking at
 * the demo workspace and clarifies WHICH kind of data the page is
 * actually showing (live empty / seeded mock-offline / mock-bypass
 * curated) so the on-screen claim matches the runtime path.
 *
 * Dismissed state persists in sessionStorage so the banner stays
 * dismissed across navigations within a single recording take, but
 * re-shows on the next fresh tab / new window. (Was localStorage; the
 * recording profile wants the banner to come back next session so the
 * operator doesn't ship an opaque "never see again" flag to investors
 * sharing the demo URL.)
 *
 * `NEXT_PUBLIC_HIDE_DEMO_BANNER=1` suppresses the banner entirely for
 * recording profiles — see `docs/skycot/demos/v1-mission-mvp-runbook.md`.
 */
export function DemoBanner() {
  const pathname = usePathname();
  const dataSource = useDataSource();
  const [hydrated, setHydrated] = useState(false);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    setHydrated(true);
    try {
      setDismissed(window.sessionStorage.getItem(DISMISS_STORAGE_KEY) === "1");
    } catch {
      setDismissed(false);
    }
    const warning = recordingModeReadinessWarning();
    if (warning) {
      // eslint-disable-next-line no-console -- intentional dev console signal
      console.warn(warning);
    }
  }, []);

  // F14: the wizard's own top bar + welcome eyebrow already signal
  // demo intent on /onboarding/*; stacking the demo banner above
  // eats ~34px of vertical real estate (especially painful at the
  // 375×812 mobile demo viewport, where the Audiowide hero gets
  // shoved below the fold).
  if (pathname?.startsWith("/onboarding")) return null;
  if (isBannerSuppressed()) return null;
  if (!isDemoMode()) return null;
  if (!hydrated) return null;
  if (dismissed) return null;

  const copy = bannerCopy(dataSource);

  // The banner is sticky-top-0 z-30 across the whole app shell. To
  // avoid intercepting pointer events on elements that scroll/render
  // underneath it (workflow-builder palette buttons, react-flow nodes
  // visible in the same y-band), the container is pointer-events:none
  // and only the dismiss button re-enables pointer events. The text is
  // informational and doesn't need interaction.
  //
  // The dismiss button gets `relative z-40` so its hit-target sits
  // above any sibling that scrolls into the banner's y-band on narrow
  // viewports (e.g. `worker-freeze-button` in
  // `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx`). Without
  // `relative` the z-index is a no-op because z-index only applies to
  // positioned elements.
  return (
    <div
      role="status"
      data-testid="demo-banner"
      data-source={dataSource ?? "unknown"}
      className="sticky top-0 z-30 border-b border-[var(--border)] bg-gradient-to-r from-[#8b5cf6]/12 via-[#ec4899]/8 to-[#8b5cf6]/12 text-[var(--foreground)] pointer-events-none"
    >
      <div className="mx-auto flex max-w-5xl items-center gap-2 px-4 py-2 sm:px-6 lg:px-8">
        <Info aria-hidden className="size-4 shrink-0 text-[#8b5cf6]" />
        <p className="flex-1 text-xs sm:text-sm">
          <span className="font-semibold">{copy.lead}</span>
          {copy.rest}
        </p>
        <button
          type="button"
          data-testid="demo-banner-dismiss"
          aria-label="Dismiss demo banner"
          onClick={() => {
            setDismissed(true);
            try {
              window.sessionStorage.setItem(DISMISS_STORAGE_KEY, "1");
            } catch {
              // sessionStorage may throw in private browsing — banner
              // just re-shows next mount.
            }
          }}
          className="relative z-40 grid size-7 place-items-center rounded-full text-[var(--muted-foreground)] hover:bg-[var(--accent)] hover:text-[var(--foreground)] transition-colors pointer-events-auto"
        >
          <X className="size-3.5" />
        </button>
      </div>
    </div>
  );
}
