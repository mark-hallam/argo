"use client";

import { useRouter } from "next/navigation";
import { useEffect } from "react";
import { HireShell } from "@/components/skycot/HireShell";
import { useHireDraft } from "@/lib/hire/use-hire-draft";

/**
 * Step 1 — Land. Placeholder body until E2-b lands.
 *
 * On mount we hydrate the draft store; if the user has progressed past
 * step 1 in a previous session and the draft is still within its 7-day
 * TTL, we route forward to wherever they left off so they don't
 * re-do work.
 */
export default function HireLandingPage() {
  const router = useRouter();
  const draft = useHireDraft();

  useEffect(() => {
    if (draft.isLoading) return;
    const saved = draft.state;
    if (!saved) return;
    if (saved.step >= 2 && saved.step <= 5) {
      router.replace(`/hire/step/${saved.step}`);
    }
  }, [draft.isLoading, draft.state, router]);

  const hasBody = Boolean(
    draft.state?.problem ||
      (draft.state?.answers && Object.keys(draft.state.answers).length > 0),
  );

  return (
    <HireShell
      step={1}
      unsavedHire={hasBody}
      continueDisabled
      onDiscard={() => void draft.clear()}
    >
      <div className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-20 px-6 text-center space-y-3">
        <h1 className="text-xl font-semibold text-[var(--foreground)]">Hire a teammate</h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          Step 1 — placeholder for E2-b (Land)
        </p>
      </div>
    </HireShell>
  );
}
