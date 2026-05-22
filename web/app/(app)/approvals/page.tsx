"use client";

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
 * Rendered as an empty-state with a clear CTA back to the live inbox
 * so a fresh-workspace user always sees next-step guidance, never a
 * dead 404.
 */
export default function ApprovalsLandingPage() {
  return (
    <main
      className="mx-auto max-w-3xl px-4 py-12 sm:px-6 lg:px-8"
      data-testid="approvals-landing"
    >
      <header className="mb-8 space-y-2">
        <p
          className="text-xs font-bold uppercase tracking-[0.16em] text-[var(--muted-foreground)]"
          aria-hidden
        >
          Inbox
        </p>
        <h1 className="text-3xl font-semibold tracking-tight text-[var(--foreground)]">
          Approvals
        </h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          Every decision your AI teammates queue for a human shows up here.
        </p>
      </header>
      <EmptyState
        data-testid="approvals-empty"
        illustration={InboxIllustration}
        headline="No approvals waiting"
        body="When a worker hits a decision that needs sign-off — a refund, a hire, a contract — it lands in your inbox at home. We'll page you in Slack the moment something arrives."
        cta={{ label: "Open the inbox", href: "/" }}
        secondary={{ label: "See your workers", href: "/workers" }}
      />
    </main>
  );
}
