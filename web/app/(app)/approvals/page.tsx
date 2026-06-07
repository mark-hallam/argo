"use client";

import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";
import { ApprovalsOpsDashboard } from "@/components/skycot/approvals/ApprovalsOpsDashboard";
import { useApprovals } from "@/lib/hooks/use-approvals";

/**
 * Standalone `/approvals` landing for deep-links and bookmarks.
 *
 * The canonical approval queue lives at `/` (the AppNav's "Approvals"
 * row routes there). This page exists so the typed URL `/approvals`
 * resolves to a useful branded surface instead of `not-found.tsx` —
 * the 2026-05-18 UX audit flagged the missing route as one of three
 * sidebar destinations that crashed for buyers in the first 30s of
 * a demo.
 *
 * 2026-05-24 design pass: swapped the plain Inter `<h1>` for the
 * shared `BrandLoudHero` so the surface matches the D3 brand-
 * consistency pass (Audiowide + purple→pink gradient) applied across
 * Cohorts / Workers / Team.
 *
 * 2026-05-26 (`fix-approvals-inbox-waiting-on-you-fixture-drift`):
 * the empty-state short-circuit is removed. The locked E-ops-
 * dashboard renders regardless of queue depth — its 0-state KPIs,
 * severity tiers and donuts already handle the "0 awaiting" case —
 * so the page stops disagreeing with /team's "Waiting on you" chip
 * count when the gateway returns zero rows.
 */
export default function ApprovalsLandingPage() {
  const approvalsQuery = useApprovals();
  const approvals = approvalsQuery.data ?? [];

  return (
    <div data-testid="approvals-landing">
      <BrandLoudHero
        eyebrowDotColor="var(--warning)"
        eyebrowLabel="INBOX"
        title="Approvals."
        sub="Every decision your AI teammates queue for a human shows up here — refunds, hires, contracts, anything that needs a sign-off."
      />
      <main className="mx-auto max-w-6xl px-4 py-10 sm:px-6 lg:px-8">
        {approvalsQuery.isPending ? (
          <DashboardLoading />
        ) : approvalsQuery.isError ? (
          <DashboardError onRetry={() => approvalsQuery.refetch()} />
        ) : (
          <ApprovalsOpsDashboard items={approvals} />
        )}
      </main>
    </div>
  );
}

function DashboardLoading() {
  return (
    <div className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] px-6 py-16 text-center">
      <p className="text-sm text-[var(--muted-foreground)]">Loading approvals...</p>
    </div>
  );
}

/**
 * Honest error state for a failed approvals fetch (e.g. a gateway timeout).
 * Distinct from the dashboard's own 0-awaiting state — a connection failure
 * must NOT read as "you have zero approvals" (a false-empty). See the
 * mock-leak / bounded-fetch work.
 */
function DashboardError({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] px-6 py-16 text-center">
      <p className="text-sm font-medium text-[var(--foreground)]">
        We couldn&rsquo;t load your approvals.
      </p>
      <p className="mt-1 text-sm text-[var(--muted-foreground)]">
        The gateway didn&rsquo;t respond — this is a connection problem, not an
        empty inbox.
      </p>
      <button
        type="button"
        onClick={onRetry}
        className="mt-4 inline-flex items-center rounded-[var(--radius-md)] border border-[var(--border)] px-4 py-2 text-sm font-medium text-[var(--foreground)] transition hover:bg-[var(--muted)]"
      >
        Retry
      </button>
    </div>
  );
}
