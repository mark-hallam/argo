"use client";

import { useRouter } from "next/navigation";
import { use, useEffect } from "react";
import { HireShell } from "@/components/skycot/HireShell";
import { useHireDraft } from "@/lib/hire/use-hire-draft";

const REDIRECT_MS = 800;

/**
 * Step 6 — celebration placeholder. Clears the draft on mount (the
 * hire flow is complete) and auto-redirects to the new worker's
 * profile after a short pause so the banner is legible.
 */
export default function HireDonePage({
  params,
}: {
  params: Promise<{ workerId: string }>;
}) {
  const { workerId } = use(params);
  const router = useRouter();
  const draft = useHireDraft();

  useEffect(() => {
    void draft.clear();
    const handle = setTimeout(() => {
      router.replace(`/workers/${workerId}`);
    }, REDIRECT_MS);
    return () => clearTimeout(handle);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workerId, router]);

  return (
    <HireShell step={6}>
      <div className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-20 px-6 text-center space-y-3">
        <h1 className="text-xl font-semibold text-[var(--foreground)]">
          Step 6 — celebration placeholder for E2-h
        </h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          Redirecting to your new teammate…
        </p>
      </div>
    </HireShell>
  );
}
