"use client";

import { Suspense, useEffect, useMemo } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Loader2 } from "lucide-react";
import { HireStarterPrompts } from "@/components/empty-state/HireStarterPrompts";
import { TeammateCard } from "@/components/skycot/TeammateCard";
import { TeammateGrid } from "@/components/skycot/TeammateGrid";
import { useTeam } from "@/lib/hooks/use-team";
import { compareWorkersForGrid } from "@/lib/types/worker";

/**
 * `/workers` — nav-bar entrypoint for the worker roster. When the
 * tenant has zero workers, render the shared `<HireStarterPrompts>`
 * empty state (the same 5 starter-prompt grid the `/team` page uses).
 * When workers exist, redirect to `/team`, the canonical roster
 * surface — `useTeam()` is the shared data source.
 *
 * The first-hire wizard exits to `/workers?just-hired=1&new=<id>` so
 * the operator sees the new card without an extra hop to `/team`; we
 * gate the `/team` redirect on `just-hired=1` and render the grid
 * inline for that one navigation. A dedicated workers list with
 * column-level filters is a separate Track-E brief.
 */
export default function WorkersPage() {
  return (
    <Suspense fallback={<WorkersFallback />}>
      <WorkersPageContent />
    </Suspense>
  );
}

function WorkersFallback() {
  return (
    <>
      <WorkersHeader />
      <div className="grid min-h-64 place-items-center" data-testid="workers-loading">
        <Loader2 className="size-5 animate-spin text-[var(--muted-foreground)]" aria-hidden />
      </div>
    </>
  );
}

function WorkersHeader() {
  return (
    <div className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
      <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-5">
        <h1 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
          Workers
        </h1>
      </div>
    </div>
  );
}

function WorkersPageContent() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const teamQuery = useTeam();
  const workers = teamQuery.data?.workers ?? [];
  const hasWorkers = !teamQuery.isPending && workers.length > 0;
  const justHired = searchParams?.get("just-hired") === "1";
  const newId = searchParams?.get("new");

  useEffect(() => {
    if (hasWorkers && !justHired) router.replace("/team");
  }, [hasWorkers, justHired, router]);

  const sortedWorkers = useMemo(
    () => [...workers].sort(compareWorkersForGrid),
    [workers],
  );

  const showGrid = hasWorkers && justHired;
  const showLoader = teamQuery.isPending || (hasWorkers && !justHired);

  return (
    <>
      <WorkersHeader />
      {showLoader ? (
        <div className="grid min-h-64 place-items-center" data-testid="workers-loading">
          <Loader2 className="size-5 animate-spin text-[var(--muted-foreground)]" aria-hidden />
        </div>
      ) : showGrid ? (
        <div
          className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8"
          data-testid="workers-grid"
        >
          <TeammateGrid>
            {sortedWorkers.map((w) => (
              <TeammateCard
                key={w.id}
                worker={w}
                className={
                  newId && w.id === newId
                    ? "ring-2 ring-[var(--primary)] ring-offset-2"
                    : undefined
                }
                data-testid={`teammate-${w.id}`}
              />
            ))}
          </TeammateGrid>
        </div>
      ) : (
        <div
          className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8 space-y-6"
          data-testid="workers-empty"
        >
          <HireStarterPrompts />
        </div>
      )}
    </>
  );
}
