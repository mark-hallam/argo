"use client";

import { useEffect, useState } from "react";
import { isDemoMode } from "@/lib/skycot/demo-mode";

const DISMISS_STORAGE_KEY = "skycot:demo-mode-banner-dismissed";

/**
 * Shell-level demo-mode banner used by the onboarding wizard (and any
 * other authenticated shell that needs to flag a demo session). Visually
 * matches the yellow caution band the signup page renders inline.
 *
 * Distinct from `DemoBanner` — that one rides the app shell after
 * onboarding ("showing sample data · sign in to see your own
 * workspace") and dismisses via localStorage. This banner is the
 * wayfinding signal for the wizard itself, persists dismissal in
 * sessionStorage so it stays gone for the duration of the wizard but
 * returns on the next session, and explains the specific affordance
 * the demo-mode flag triggers: first-hire creates a staged teammate,
 * provider calls are simulated.
 */
export function DemoModeBanner() {
  const [hydrated, setHydrated] = useState(false);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    setHydrated(true);
    try {
      setDismissed(window.sessionStorage.getItem(DISMISS_STORAGE_KEY) === "1");
    } catch {
      setDismissed(false);
    }
  }, []);

  if (!isDemoMode()) return null;
  if (!hydrated) return null;
  if (dismissed) return null;

  return (
    <div
      role="status"
      data-testid="demo-mode-banner"
      style={{
        background: "#fef3c7",
        borderBottom: "1px solid #f59e0b",
        color: "#78350f",
        padding: "10px 16px",
        display: "flex",
        alignItems: "center",
        gap: 12,
        fontSize: 13,
        fontWeight: 600,
        position: "relative",
        zIndex: 11,
      }}
    >
      <span style={{ flex: 1, textAlign: "center" }}>
        Demo mode — first-hire creates a staged teammate, not a real
        provisioning. Provider calls are simulated.
      </span>
      <button
        type="button"
        data-testid="demo-mode-banner-dismiss"
        aria-label="Dismiss demo mode banner"
        onClick={() => {
          setDismissed(true);
          try {
            window.sessionStorage.setItem(DISMISS_STORAGE_KEY, "1");
          } catch {
            // sessionStorage may throw in private browsing — banner
            // just re-shows next mount.
          }
        }}
        style={{
          background: "transparent",
          border: "none",
          color: "#78350f",
          fontSize: 18,
          lineHeight: 1,
          cursor: "pointer",
          padding: "0 4px",
        }}
      >
        ×
      </button>
    </div>
  );
}
