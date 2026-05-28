"use client";

import { HoverHint } from "@/components/ui/Tooltip";
import { resolveGatewayUrl } from "@/lib/config/gateway-url";
import { isDemoMode } from "@/lib/skycot/demo-mode";

/**
 * Always-visible sidebar footer marker that tells the operator
 * whether the dev server is rendering seeded demo data or talking to
 * the real gateway. Solves the recurring recording-day foot-gun
 * documented in `fix-sidebar-live-vs-demo-mode-stamp`: a fresh
 * terminal that runs `npm run dev` without re-exporting
 * `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` silently launches in honest mode,
 * but the chrome looks identical and Scene 3 of the demo runbook
 * breaks halfway through. The stamp puts the answer on screen
 * before the operator hits record.
 *
 * Hidden in production builds — production should only ever be
 * live, so the dot would be noise on the buyer's chrome.
 */
export function ModeStamp() {
  if (process.env.NODE_ENV === "production") return null;
  const demo = isDemoMode();
  const mode = demo ? "demo" : "live";
  const label = demo ? "Demo mode" : "Live mode";
  const gateway = resolveGatewayUrl();
  const tooltip = demo
    ? "Showing sample data. Set NEXT_PUBLIC_SKYCOT_DEMO_MODE=0 and restart the dev server to go live."
    : `Connected to gateway at ${gateway}.`;
  const dotColor = demo ? "#8b5cf6" : "#10b981";
  return (
    <HoverHint label={tooltip} side="right">
      <div
        data-testid="sidebar-mode-stamp"
        data-mode={mode}
        className="mt-1 mx-2 flex items-center gap-1.5 px-2 py-1 text-[11px] text-[var(--muted-foreground)] select-none cursor-default"
      >
        <span
          aria-hidden
          data-testid="sidebar-mode-stamp-dot"
          className="inline-block size-1.5 rounded-full"
          style={{ background: dotColor }}
        />
        <span>{label}</span>
      </div>
    </HoverHint>
  );
}
