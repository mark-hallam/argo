"use client";

import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";
import { ApprovalsOpsDashboard } from "@/components/skycot/approvals/ApprovalsOpsDashboard";
import { EmptyState } from "@/components/ui/EmptyState";
import { InboxIllustration } from "@/components/empty-state/illustrations/InboxIllustration";
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
 * Cohorts / Workers / Team. Pending approvals now render the locked
 * ops-dashboard body; fresh workspaces keep the CTA empty state.
 */
export default function ApprovalsLandingPage() {
  const approvalsQuery = useApprovals();
  const approvals = approvalsQuery.data ?? [];
  const pendingCount = approvals.filter((item) => item.decision == null).length;

  return (
    <div data-testid="approvals-landing">
      <BrandLoudHero
        eyebrowDotColor="#fbbf24"
        eyebrowLabel="INBOX"
        title="Approvals."
        sub="Every decision your AI teammates queue for a human shows up here — refunds, hires, contracts, anything that needs a sign-off."
      />
      <main className="mx-auto max-w-6xl px-4 py-10 sm:px-6 lg:px-8">
        {approvalsQuery.isPending ? (
          <DashboardLoading />
        ) : pendingCount === 0 ? (
          <ApprovalsEmptyState />
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

function ApprovalsEmptyState() {
  return (
    <div className="mx-auto max-w-3xl">
      <EmptyState
        data-testid="approvals-empty"
        illustration={InboxIllustration}
        headline="No approvals waiting"
        body="When a worker hits a decision that needs sign-off — a refund, a hire, a contract — it lands in your inbox at home. We'll page you in Slack the moment something arrives."
        cta={{ label: "Open the inbox", href: "/" }}
        secondary={{ label: "See your workers", href: "/workers" }}
      />
    </div>
  );
}
