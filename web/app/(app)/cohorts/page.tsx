"use client";

import { useCallback, useState } from "react";
import { CohortCard } from "@/components/skycot/CohortCard";
import { CohortDetailDrawer } from "@/components/skycot/CohortDetailDrawer";
import { useCohorts } from "@/lib/hooks/use-cohorts";

export default function CohortsPage() {
  const cohortsQuery = useCohorts();
  const [openCohortId, setOpenCohortId] = useState<string | null>(null);

  const onSelect = useCallback((id: string) => setOpenCohortId(id), []);
  const onClose = useCallback(() => setOpenCohortId(null), []);

  const cohorts = cohortsQuery.data?.cohorts ?? [];
  const total = cohorts.length;

  return (
    <>
      <div className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
        <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-5">
          <div className="space-y-1">
            <h1 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
              Cohorts
            </h1>
            <p
              className="text-sm text-[var(--muted-foreground)]"
              data-testid="cohorts-subtitle"
            >
              {cohortsQuery.isPending ? (
                "Loading cohorts…"
              ) : (
                <>
                  {total} {total === 1 ? "cohort" : "cohorts"} across the fleet
                </>
              )}
            </p>
          </div>
        </div>
      </div>

      <div
        className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8"
        data-testid="cohorts-page"
      >
        {cohortsQuery.isPending ? (
          <LoadingGrid />
        ) : cohorts.length === 0 ? (
          <EmptyState />
        ) : (
          <div
            className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4"
            data-testid="cohort-grid"
            role="list"
          >
            {cohorts.map((c) => (
              <div role="listitem" key={c.cohort_id}>
                <CohortCard
                  cohort={c}
                  onSelect={onSelect}
                  selected={c.cohort_id === openCohortId}
                />
              </div>
            ))}
          </div>
        )}
      </div>

      <CohortDetailDrawer cohortId={openCohortId} onClose={onClose} />
    </>
  );
}

function LoadingGrid() {
  return (
    <div
      data-testid="cohorts-loading"
      className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4"
    >
      {Array.from({ length: 6 }).map((_, i) => (
        <div
          key={i}
          className="h-40 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)]/40 animate-pulse"
        />
      ))}
    </div>
  );
}

function EmptyState() {
  return (
    <div
      data-testid="cohorts-empty"
      className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-16 px-6 text-center space-y-2"
    >
      <h2 className="text-base font-semibold text-[var(--foreground)]">
        No cohorts yet
      </h2>
      <p className="text-sm text-[var(--muted-foreground)]">
        Cohorts appear once you have workers in a vertical sharing a trust
        grade. Hire a teammate or assign skills to existing ones.
      </p>
    </div>
  );
}
