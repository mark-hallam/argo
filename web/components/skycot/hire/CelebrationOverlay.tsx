"use client";

import { PartyPopper } from "lucide-react";

export interface CelebrationOverlayProps {
  workerName: string;
}

/**
 * Mid-screen celebration card shown on `/hire/done/<workerId>` for
 * ~1.5s before the router redirects to the approvals inbox. Pure
 * presentation — the parent owns the redirect timer.
 *
 * Copy is locked by the E2-h brief: `"🎉 ${workerName} is hired."` +
 * tagline `"Heading to your inbox..."`. Don't drift.
 */
export function CelebrationOverlay({ workerName }: CelebrationOverlayProps) {
  return (
    <div
      className="flex flex-col items-center justify-center gap-4 py-24 text-center"
      data-testid="hire-celebration-overlay"
    >
      <div
        className="grid size-16 place-items-center rounded-full bg-[var(--primary)]/10 text-[var(--primary)]"
        aria-hidden
      >
        <PartyPopper className="size-8" />
      </div>
      <h1
        className="text-2xl font-semibold text-[var(--foreground)]"
        aria-live="polite"
      >
        <span aria-hidden>🎉 </span>
        {workerName} is hired.
      </h1>
      <p className="text-sm text-[var(--muted-foreground)]">
        Heading to your inbox…
      </p>
    </div>
  );
}
