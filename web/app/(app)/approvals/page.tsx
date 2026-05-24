"use client";

import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";
import { EmptyState } from "@/components/ui/EmptyState";
import { InboxIllustration } from "@/components/empty-state/illustrations/InboxIllustration";

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
 * Cohorts / Workers / Team. The empty-state copy below remains the
 * primary user guidance.
 */
export default function ApprovalsLandingPage() {
  return (
    <div data-testid="approvals-landing">
      <BrandLoudHero
        eyebrowDotColor="#fbbf24"
        eyebrowLabel="INBOX"
        title="Approvals."
        sub="Every decision your AI teammates queue for a human shows up here — refunds, hires, contracts, anything that needs a sign-off."
      />
      <main className="mx-auto max-w-3xl px-4 py-10 sm:px-6 lg:px-8">
        <EmptyState
          data-testid="approvals-empty"
          illustration={InboxIllustration}
          headline="No approvals waiting"
          body="When a worker hits a decision that needs sign-off — a refund, a hire, a contract — it lands in your inbox at home. We'll page you in Slack the moment something arrives."
          cta={{ label: "Open the inbox", href: "/" }}
          secondary={{ label: "See your workers", href: "/workers" }}
        />
      </main>
    </div>
  );
}
