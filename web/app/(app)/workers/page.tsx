"use client";

import { Suspense, useEffect, useMemo } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Loader2 } from "lucide-react";
import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";
import { HireStarterPrompts } from "@/components/empty-state/HireStarterPrompts";
import { TeammateCard } from "@/components/skycot/TeammateCard";
import { TeammateGrid } from "@/components/skycot/TeammateGrid";
import { useTeam } from "@/lib/hooks/use-team";
import { compareWorkersForGrid, type Worker } from "@/lib/types/worker";
import type { UseQueryResult } from "@tanstack/react-query";
import type { FetchTeamResult } from "@/lib/api/team";

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
 *
 * `useTeam()` is called HERE, in the OUTER component, ABOVE the
 * `useSearchParams()` Suspense boundary below — the same hoist the
 * `/team` Scene-1 fix used (`web/app/(app)/team/page.tsx`). When that
 * boundary suspended on hydration inside `RouteTransition`'s
 * `<AnimatePresence>` (`lib/motion/primitives.tsx`), a fetch mounted
 * *inside* it never had its React Query observer subscribe, so
 * `queryFn` never fired: `/workers` showed "Fetching the live worker
 * roster." with NO `/api/v1/workers` request, and the `/team` redirect
 * (also gated by the boundary) never ran either. Mounting `useTeam()`
 * here guarantees the fetch fires on mount regardless of the inner
 * boundary; the `just-hired`/`new` URL params stay inside the boundary.
 */
export default function WorkersPage() {
  const teamQuery = useTeam();
  const workers = useMemo(
    () => (Array.isArray(teamQuery.data?.workers) ? teamQuery.data.workers : []),
    [teamQuery.data],
  );
  return (
    <Suspense fallback={<WorkersFallback teamQuery={teamQuery} workers={workers} />}>
      <WorkersPageContent teamQuery={teamQuery} workers={workers} />
    </Suspense>
  );
}

// Suspense fallback — shown only for the brief window while
// `useSearchParams()` resolves. Renders from the already-mounted
// `teamQuery` (never gated behind the boundary): loader while pending,
// otherwise the empty "hire" grid. The just-hired highlight is the only
// thing that needs the URL, so it's deferred to the committed view.
function WorkersFallback({
  teamQuery,
  workers,
}: {
  teamQuery: UseQueryResult<FetchTeamResult>;
  workers: Worker[];
}) {
  const hasWorkers = !teamQuery.isPending && workers.length > 0;
  if (teamQuery.isPending || hasWorkers) {
    return (
      <>
        <WorkersHero state={hasWorkers ? "redirecting" : "loading"} total={workers.length} />
        <div className="grid min-h-64 place-items-center" data-testid="workers-loading">
          <Loader2 className="size-5 animate-spin text-[var(--muted-foreground)]" aria-hidden />
        </div>
      </>
    );
  }
  return (
    <>
      <WorkersHero state="empty" />
      <div
        className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8 space-y-6"
        data-testid="workers-empty"
      >
        <HireStarterPrompts />
      </div>
    </>
  );
}

function WorkersHero({
  state,
  total = 0,
}: {
  state: "loading" | "empty" | "just-hired" | "redirecting";
  total?: number;
}) {
  const sub =
    state === "empty"
      ? "Pick a starting point to hire a teammate with persistent memory, skills, audit trails, and approval gates."
      : state === "just-hired"
        ? "Review the new worker, then open their profile to inspect memory, skills, trust, and settings."
      : state === "redirecting"
        ? "Opening the canonical Team roster."
        : "Fetching the live worker roster.";

  return (
    <BrandLoudHero
      eyebrowDotColor="var(--brand-primary-foreground)"
      eyebrowLabel="WORKERS · C2 ROSTER"
      title="Workers"
      sub={sub}
      subMaxWidth={520}
      shineDurationS={4.8}
      kpis={
        total > 0
          ? [
              { label: "Teammates", value: total.toLocaleString() },
            ]
          : undefined
      }
    />
  );
}

function WorkersPageContent({
  teamQuery,
  workers,
}: {
  teamQuery: UseQueryResult<FetchTeamResult>;
  workers: Worker[];
}) {
  const router = useRouter();
  const searchParams = useSearchParams();
  // `workers` is fetched by the parent and passed down — see the
  // `WorkersPage` doc-comment for why the fetch lives above this boundary.
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
      <WorkersHero
        state={
          showLoader
            ? hasWorkers
              ? "redirecting"
              : "loading"
            : showGrid
              ? "just-hired"
              : "empty"
        }
        total={sortedWorkers.length}
      />
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
