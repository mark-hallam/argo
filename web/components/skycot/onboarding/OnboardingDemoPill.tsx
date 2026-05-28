"use client";

import { isDemoMode } from "@/lib/skycot/demo-mode";

/**
 * Low-key bottom-right pill that appears inside the onboarding wizard
 * shell when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`. The gradient `<DemoBanner />`
 * mounts only inside `(app)/layout.tsx`, so during the pre-record dress
 * rehearsal the operator had no visual cue confirming the wizard pages
 * were running in demo mode. The full banner is intentionally NOT used
 * here — Scene 3 of the v1.0 demo MP4 records the wizard and the
 * brand-gradient bar would compete with the F1 editorial-center hero.
 *
 * Renders nothing outside demo mode so it is a no-op for real users.
 */
export function OnboardingDemoPill() {
  if (!isDemoMode()) return null;
  return (
    <div
      role="status"
      aria-label="Demo mode active"
      data-testid="onboarding-demo-pill"
      className="pointer-events-none fixed bottom-3 right-3 z-40 rounded-full border border-amber-300/70 bg-amber-100/90 px-3 py-1.5 text-[11px] font-semibold uppercase tracking-wide text-amber-900 shadow-sm backdrop-blur-sm"
    >
      Demo mode · sample data only
    </div>
  );
}
