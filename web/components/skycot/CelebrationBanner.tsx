"use client";

import Link from "next/link";
import { ArrowRight, PartyPopper, X } from "lucide-react";
import { useCelebrations } from "@/lib/hire/use-celebrations";
import type { CelebrationEntry } from "@/lib/hire/celebration-storage";

/**
 * Dismissible celebration banner shown above the approvals inbox header
 * for ~24h after a hire commit. Renders the most-recent unconsumed
 * celebration (max 1 visible at a time); when the user dismisses it,
 * the next-most-recent entry surfaces. When the freshly-hired worker
 * produces their first approval card, the hook auto-clears the
 * celebration (the card itself replaces the banner as the "first
 * opinion" signal).
 *
 * Two variants:
 * - **Active** (default): full hand-off copy + profile link.
 * - **Inert** (`?inert=1` skipped Step 5 Connect): copy nudges the
 *   user to connect tools; CTA jumps to the worker's settings tab.
 *
 * Renders nothing on the server / first render so localStorage reads
 * stay client-side. Hydration mismatch is intentionally avoided
 * because `useCelebrations` returns `[]` until the first `useEffect`
 * runs.
 */
export function CelebrationBanner() {
  const { entries, dismiss } = useCelebrations();
  if (entries.length === 0) return null;

  // Most recent at the tail of the oldest-first list. One banner at a time.
  const current = entries[entries.length - 1];
  return (
    <BannerLayout>
      <BannerCard entry={current} onDismiss={() => dismiss(current.workerId)} />
    </BannerLayout>
  );
}

/**
 * Test-only variant that lets unit tests inject controlled props
 * without going through the localStorage-backed hook + approvals
 * query. Production code paths always render `<CelebrationBanner />`.
 */
export function CelebrationBannerView({
  entries,
  onDismiss,
}: {
  entries: CelebrationEntry[];
  onDismiss: (workerId: string) => void;
}) {
  if (entries.length === 0) return null;
  const current = entries[entries.length - 1];
  return (
    <BannerLayout>
      <BannerCard entry={current} onDismiss={() => onDismiss(current.workerId)} />
    </BannerLayout>
  );
}

function BannerLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 pt-4">{children}</div>
  );
}

function BannerCard({
  entry,
  onDismiss,
}: {
  entry: CelebrationEntry;
  onDismiss: () => void;
}) {
  const isInert = entry.inert;
  const profileHref = isInert
    ? `/workers/${encodeURIComponent(entry.workerId)}?tab=settings`
    : `/workers/${encodeURIComponent(entry.workerId)}`;
  const ctaLabel = isInert
    ? "Connect tools"
    : `Open ${entry.workerName}'s profile`;
  return (
    <div
      className="relative flex items-center gap-3 rounded-[var(--radius-md)] border-l-[3px] border-l-[var(--primary)] bg-[color-mix(in_oklab,var(--primary)_8%,var(--background))] py-2 pl-3 pr-10 text-sm shadow-sm"
      style={{ maxHeight: 56 }}
      data-testid="celebration-banner"
      data-variant={isInert ? "inert" : "active"}
      role="status"
      aria-live="polite"
    >
      <span
        className="grid size-7 shrink-0 place-items-center rounded-full bg-[var(--primary)]/15 text-[var(--primary)]"
        aria-hidden
      >
        <PartyPopper className="size-3.5" />
      </span>
      <p className="flex-1 truncate text-[var(--foreground)]">
        {isInert ? (
          <>
            <span className="font-medium">{entry.workerName}</span> is hired
            but needs tools to start working.{" "}
          </>
        ) : (
          <>
            <span aria-hidden>🎉 </span>
            <span className="font-medium">{entry.workerName}</span> is hired.
            She&apos;ll start working in the background. The first time she has an
            opinion, you&apos;ll see it here.{" "}
          </>
        )}
        <Link
          href={profileHref}
          className="inline-flex items-center gap-1 font-medium text-[var(--primary)] underline-offset-2 hover:underline"
          data-testid="celebration-banner-cta"
        >
          {ctaLabel}
          <ArrowRight className="size-3.5" aria-hidden />
        </Link>
      </p>
      <button
        type="button"
        onClick={onDismiss}
        aria-label={`Dismiss celebration for ${entry.workerName}`}
        className="absolute right-2 top-1/2 -translate-y-1/2 grid size-7 place-items-center rounded-full text-[var(--muted-foreground)] hover:bg-[var(--muted)] hover:text-[var(--foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)]"
        data-testid="celebration-banner-dismiss"
      >
        <X className="size-4" />
      </button>
    </div>
  );
}
