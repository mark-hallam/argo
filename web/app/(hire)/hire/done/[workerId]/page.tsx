"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, use, useEffect, useState } from "react";
import { HireShell } from "@/components/skycot/HireShell";
import { CelebrationOverlay } from "@/components/skycot/hire/CelebrationOverlay";
import { setCelebration } from "@/lib/hire/celebration-storage";
import { useHireDraft } from "@/lib/hire/use-hire-draft";

const REDIRECT_MS = 1500;

/**
 * Step 6 — celebration handoff. Stamps a CelebrationEntry into local
 * storage so the approvals inbox can surface the banner on the next
 * mount, renders a 1.5s mid-screen celebration overlay, clears the
 * hire draft, then redirects to `/`.
 *
 * `?inert=1` (set by Step 5's "Skip for now") makes the banner show
 * the inert variant ("X is hired but needs tools to start working").
 * `?name=…` is a fallback when the draft has been cleared in another
 * tab — the URL is the single carrier of identity in that case.
 *
 * 1.5s is locked by the brief — longer feels like a delay, not a
 * celebration. The overlay carries `workerName` so the visual matches
 * the worker the user just hired.
 */
export default function HireDonePage({
  params,
}: {
  params: Promise<{ workerId: string }>;
}) {
  const { workerId } = use(params);
  return (
    <Suspense fallback={null}>
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

  useEffect(() => {
    if (draft.isLoading) return;
    const name =
      draft.state?.workerDraft?.suggestedName ??
      nameFromQuery ??
      "Your teammate";
    setResolvedName(name);

    setCelebration(workerId, { workerName: name, inert: isInert });
    void draft.clear();
    const handle = setTimeout(() => {
      router.replace("/");
    }, REDIRECT_MS);
    return () => clearTimeout(handle);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workerId, isInert, draft.isLoading]);

  return (
    <HireShell step={6}>
      <CelebrationOverlay workerName={resolvedName} />
    </HireShell>
  );
}
