"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, use, useEffect, useState } from "react";
import { WizardSkeleton } from "@/components/auth/WizardSkeleton";
import { WizardErrorState } from "@/components/auth/WizardErrorState";
import { HireDoneCelebration } from "@/components/skycot/hire/HireDoneCelebration";
import { setCelebration } from "@/lib/hire/celebration-storage";
import { useHireDraft } from "@/lib/hire/use-hire-draft";

/**
 * Step 6 — viral-moment payoff. Lands the newly-hired worker on a
 * brand-loud BrandLoudHero (`Meet ${name}.`) plus confetti burst and a
 * welcome card with Slack + View-profile CTAs.
 *
 * Per the `web-hire-done-celebration-brandloud` brief the auto-redirect
 * from the prior celebration overlay is removed — the page IS the
 * payoff, and the user chooses where to go via the CTAs. The
 * `setCelebration()` write into local storage is kept so the home /
 * approvals banner still surfaces on later visits.
 *
 * `?inert=1` (set by Step 5's "Skip for now") still flows into the
 * stored celebration entry so the banner can show the inert variant
 * later. `?name=…` is the fallback identity carrier when the draft has
 * been cleared in another tab.
 */
export default function HireDonePage({
  params,
}: {
  params: Promise<{ workerId: string }>;
}) {
  const { workerId } = use(params);
  return (
    <Suspense fallback={<WizardSkeleton step={6} />}>
      <HireDoneContent workerId={workerId} />
    </Suspense>
  );
}

function HireDoneContent({ workerId }: { workerId: string }) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const draft = useHireDraft();
  const isInert = searchParams?.get("inert") === "1";
  const nameFromQuery = searchParams?.get("name") ?? null;
  const [resolvedName, setResolvedName] = useState<string>("Your teammate");
  const [resolvedRole, setResolvedRole] = useState<string | undefined>(undefined);

  useEffect(() => {
    if (draft.isLoading) return;
    if (draft.error) return;
    const name =
      draft.state?.workerDraft?.suggestedName ??
      nameFromQuery ??
      "Your teammate";
    const role =
      draft.state?.workerDraft?.persona?.tagline ??
      draft.state?.workerDraft?.suggestedVertical ??
      undefined;
    setResolvedName(name);
    setResolvedRole(role);

    setCelebration(workerId, { workerName: name, inert: isInert });
    void draft.clear();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workerId, isInert, draft.isLoading, draft.error]);

  if (draft.isLoading) {
    return <WizardSkeleton step={6} />;
  }

  if (draft.error) {
    return (
      <WizardErrorState
        variant="worker-fetch"
        errorMessage={draft.error.message}
        onReset={() => {
          void draft.clear();
          router.replace("/team");
        }}
      />
    );
  }

  return (
    <HireDoneCelebration
      workerId={workerId}
      workerName={resolvedName}
      role={resolvedRole}
    />
  );
}
