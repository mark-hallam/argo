"use client";

import { Info, X } from "lucide-react";
import { useEffect, useState } from "react";
import { isDemoMode } from "@/lib/skycot/demo-mode";

const DISMISS_STORAGE_KEY = "skycot:demo-banner-dismissed";

/**
 * Slim banner that pins to the top of the app shell whenever
 * `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`. Tells the viewer they're looking at
 * seeded sample data (Sam Harlow · Acmeco · six fake teammates), not
 * their own workspace. Dismissed state persists in localStorage so the
 * banner doesn't reappear after every navigation during the recorded
 * demo, but it re-shows after the operator clears storage to test the
 * default branch.
 */
export function DemoBanner() {
  const [hydrated, setHydrated] = useState(false);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    setHydrated(true);
    try {
      setDismissed(window.localStorage.getItem(DISMISS_STORAGE_KEY) === "1");
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
      data-testid="demo-banner"
      className="sticky top-0 z-30 border-b border-[var(--border)] bg-gradient-to-r from-[#8b5cf6]/12 via-[#ec4899]/8 to-[#8b5cf6]/12 text-[var(--foreground)]"
    >
      <div className="mx-auto flex max-w-5xl items-center gap-2 px-4 py-2 sm:px-6 lg:px-8">
        <Info aria-hidden className="size-4 shrink-0 text-[#8b5cf6]" />
        <p className="flex-1 text-xs sm:text-sm">
          <span className="font-semibold">Demo mode —</span> showing
          sample data. Sign in to see your own workspace.
        </p>
        <button
          type="button"
          data-testid="demo-banner-dismiss"
          aria-label="Dismiss demo banner"
          onClick={() => {
            setDismissed(true);
            try {
              window.localStorage.setItem(DISMISS_STORAGE_KEY, "1");
            } catch {
              // localStorage may throw in private browsing — banner just
              // re-shows next mount.
            }
          }}
          className="grid size-7 place-items-center rounded-full text-[var(--muted-foreground)] hover:bg-[var(--accent)] hover:text-[var(--foreground)] transition-colors"
        >
          <X className="size-3.5" />
        </button>
      </div>
    </div>
  );
}
