"use client";

import { useCallback, useState } from "react";
import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";
import { CohortCard } from "@/components/skycot/CohortCard";
import { CohortDetailDrawer } from "@/components/skycot/CohortDetailDrawer";
import { EmptyState } from "@/components/ui/EmptyState";
import { CohortsIllustration } from "@/components/empty-state/illustrations/CohortsIllustration";
import { useCohorts } from "@/lib/hooks/use-cohorts";

export default function CohortsPage() {
  const cohortsQuery = useCohorts();
  const [openCohortId, setOpenCohortId] = useState<string | null>(null);

  const onSelect = useCallback((id: string) => setOpenCohortId(id), []);
  const onClose = useCallback(() => setOpenCohortId(null), []);

  const cohorts = cohortsQuery.data?.cohorts ?? [];
  const total = cohorts.length;

  // Cohort aggregate KPIs for the brand-loud hero banner.
  // CohortSummary uses snake_case fields (member_count / skill_count).
  const totalMembers = cohorts.reduce((sum, c) => sum + (c.member_count ?? 0), 0);
  const totalSkills = cohorts.reduce((sum, c) => sum + (c.skill_count ?? 0), 0);

  return (
    <>
      <CohortsHero total={total} members={totalMembers} skills={totalSkills} />

      <div className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
        <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-5">
          <div className="space-y-1">
            <h3 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
              Cohorts
            </h3>
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
          <EmptyState
            data-testid="cohorts-empty"
            illustration={CohortsIllustration}
            headline="No cohorts"
            body="Group similar tasks into a cohort to share context across workers."
            cta={{ label: "Create cohort", href: "/cohorts/new" }}
          />
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

/**
 * Cohorts hero — D3 magazine-tiles brand-loud banner.
 *
 * Wraps the shared `<BrandLoudHero>` with the cohorts-specific copy
 * and KPIs. The locked D3 magazine-tiles design lives in the cohort
 * grid below; the hero gives the page its "Bloomberg-feature
 * editorial" register. The eyebrow dot does not pulse — cohorts read
 * as a stable taxonomy rather than a live feed.
 */
function CohortsHero({ total, members, skills }: { total: number; members: number; skills: number }) {
  let title: string;
  let sub: string;
  if (total === 0) {
    title = "Build your first cohort.";
    sub = "Cohorts group teammates by department, skill family, or trust grade.";
  } else if (total === 1) {
    title = "1 cohort across your fleet.";
    sub = "Trust grades earn workers into more responsibility over time.";
  } else {
    title = `${total} cohorts across your fleet.`;
    sub = "Each cohort earns its trust grade by shipping consistently. Promotions happen automatically.";
  }
  return (
    <BrandLoudHero
      eyebrowDotColor="#ddd6fe"
      eyebrowLabel="COHORTS"
      title={title}
      sub={sub}
      subMaxWidth={480}
      shineDurationS={6}
      kpis={
        total > 0
          ? [
              { label: "Cohorts", value: String(total) },
              { label: "Members", value: String(members) },
              { label: "Skills", value: String(skills) },
            ]
          : undefined
      }
    />
  );
}

