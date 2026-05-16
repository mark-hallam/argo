"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { Loader2 } from "lucide-react";
import { EmptyState } from "@/components/ui/EmptyState";
import { WorkersIllustration } from "@/components/empty-state/illustrations/WorkersIllustration";
import { useTeam } from "@/lib/hooks/use-team";

/**
 * `/workers` — nav-bar entrypoint for the worker roster. When the
 * tenant has zero workers, render the unified empty state and invite
 * the operator to hire their first AI teammate. When workers exist,
 * redirect to `/team`, the canonical roster surface — `useTeam()` is
 * the shared data source. A dedicated workers list with column-level
 * filters is a separate Track-E brief; this page only owns the
 * empty-state experience until then.
 */
export default function WorkersPage() {
  const router = useRouter();
  const teamQuery = useTeam();
  const workers = teamQuery.data?.workers ?? [];
  const hasWorkers = !teamQuery.isPending && workers.length > 0;

  useEffect(() => {
    if (hasWorkers) router.replace("/team");
  }, [hasWorkers, router]);

  if (teamQuery.isPending || hasWorkers) {
    return (
      <div className="grid min-h-64 place-items-center" data-testid="workers-loading">
        <Loader2 className="size-5 animate-spin text-[var(--muted-foreground)]" aria-hidden />
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8">
      <EmptyState
        data-testid="workers-empty"
        illustration={WorkersIllustration}
        headline="No teammates yet"
        body="Hire your first AI worker to start shipping."
        cta={{ label: "Hire a worker", href: "/hire" }}
      />
    </div>
  );
}
