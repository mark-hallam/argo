"use client";

import Link from "next/link";
import { useCallback, useMemo, useState } from "react";
import { BrandLoudHero } from "@/components/skycot/BrandLoudHero";
import { CohortDeptTabs } from "@/components/skycot/cohorts/CohortDeptTabs";
import { CohortMagazineGrid } from "@/components/skycot/cohorts/CohortMagazineGrid";
import { CohortMagazineSkeleton } from "@/components/skycot/cohorts/CohortMagazineSkeleton";
import { CohortMagazineStyles } from "@/components/skycot/cohorts/CohortMagazineStyles";
import {
  groupCohortsByDepartment,
  type DeptGroup,
} from "@/components/skycot/cohorts/dept-registry";
import { CohortDetailDrawer } from "@/components/skycot/CohortDetailDrawer";
import { buttonVariants } from "@/components/ui/Button";
import { useCohorts } from "@/lib/hooks/use-cohorts";
import { SAMPLE_COHORT_SUMMARIES } from "@/lib/sample-cohorts";

const EMPTY_PREVIEW_GROUPS = groupCohortsByDepartment(SAMPLE_COHORT_SUMMARIES);

/**
 * /cohorts — locked D3 magazine-tile editorial layout.
 *
 * Replaces the previous plain `<Card>` grid with the locked design:
 * 6+ dept tabs at the top, then an asymmetric magazine grid per
 * department (1 lead tile + 2 stacked + 4 compact, adaptive when the
 * dept has fewer cohorts). Lead tile carries the brand-loud register
 * (Audiowide hero copy, purple→pink gradient accent); stacked/compact
 * tiles stay editorial-restrained. Clicking any tile opens the
 * existing `CohortDetailDrawer`.
 */
export default function CohortsPage() {
  const cohortsQuery = useCohorts();
  const [openCohortId, setOpenCohortId] = useState<string | null>(null);

  const onSelect = useCallback((id: string) => setOpenCohortId(id), []);
  const onClose = useCallback(() => setOpenCohortId(null), []);

  const cohorts = cohortsQuery.data?.cohorts ?? [];
  const total = cohorts.length;

  // Aggregate KPIs for the brand-loud hero. CohortSummary uses snake_case.
  const totalMembers = cohorts.reduce((sum, c) => sum + (c.member_count ?? 0), 0);
  const totalSkills = cohorts.reduce((sum, c) => sum + (c.skill_count ?? 0), 0);

  // Group by department for the magazine layout. Memoised so the tab
  // state and selection don't re-bucket cohorts on every render.
  const groups = useMemo(() => groupCohortsByDepartment(cohorts), [cohorts]);
  const [activeDeptId, setActiveDeptId] = useState<string | null>(null);
  const resolvedActive =
    activeDeptId && groups.some((g) => g.dept.id === activeDeptId)
      ? activeDeptId
      : (groups[0]?.dept.id ?? null);
  const activeGroup =
    groups.find((g) => g.dept.id === resolvedActive) ?? groups[0] ?? null;
  const previewActive =
    activeDeptId && EMPTY_PREVIEW_GROUPS.some((g) => g.dept.id === activeDeptId)
      ? activeDeptId
      : (EMPTY_PREVIEW_GROUPS[0]?.dept.id ?? "");
  const previewGroup =
    EMPTY_PREVIEW_GROUPS.find((g) => g.dept.id === previewActive) ??
    EMPTY_PREVIEW_GROUPS[0] ??
    null;

  return (
    <>
      <CohortsHero total={total} members={totalMembers} skills={totalSkills} />
      <CohortMagazineStyles />

      <div className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
        <div className="mx-auto max-w-6xl px-4 sm:px-6 lg:px-8 py-5">
          <div className="space-y-1">
            <h2 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
              Cohorts
            </h2>
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
        className="mx-auto max-w-6xl px-4 sm:px-6 lg:px-8 py-8"
        data-testid="cohorts-page"
      >
        {cohortsQuery.isPending ? (
          <CohortMagazineSkeleton />
        ) : cohorts.length === 0 ? (
          <CohortsEmptyPreview
            groups={EMPTY_PREVIEW_GROUPS}
            activeId={previewActive}
            activeGroup={previewGroup}
            onDeptChange={setActiveDeptId}
          />
        ) : (
          <div data-testid="cohort-grid" role="region" aria-label="Cohorts by department">
            <CohortDeptTabs
              groups={groups}
              activeId={resolvedActive ?? ""}
              onChange={setActiveDeptId}
            />
            {activeGroup ? (
              <CohortMagazineGrid
                group={activeGroup}
                onSelect={onSelect}
                selectedId={openCohortId}
              />
            ) : null}
          </div>
        )}
      </div>

      <CohortDetailDrawer cohortId={openCohortId} onClose={onClose} />
    </>
  );
}

function CohortsEmptyPreview({
  groups,
  activeId,
  activeGroup,
  onDeptChange,
}: {
  groups: DeptGroup[];
  activeId: string;
  activeGroup: DeptGroup | null;
  onDeptChange: (id: string) => void;
}) {
  return (
    <section
      className="space-y-6"
      data-testid="cohorts-empty"
      aria-label="Cohort layout preview"
    >
      <div className="flex flex-col gap-4 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 text-[var(--card-foreground)] shadow-sm md:flex-row md:items-center md:justify-between">
        <div className="space-y-2">
          <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.16em] text-[var(--muted-foreground)]">
            Preview examples
          </p>
          <div className="space-y-1">
            <h2 className="text-lg font-semibold tracking-tight">
              Create your first cohort
            </h2>
            <p className="max-w-2xl text-sm leading-6 text-[var(--muted-foreground)]">
              Your workspace has no cohort rows yet. The marked examples below show
              how departments will arrange once real workers are assigned.
            </p>
          </div>
        </div>
        <Link
          href="/cohorts/new"
          className={buttonVariants({ size: "sm" })}
          data-testid="cohorts-create-cta"
        >
          Create cohort
        </Link>
      </div>

      <div
        className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] p-4 sm:p-5"
        data-testid="cohort-preview-grid"
      >
        <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
          <span className="rounded-[var(--radius-sm)] bg-[var(--muted)] px-2 py-1 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-[var(--muted-foreground)]">
            Example data
          </span>
          <span className="text-xs text-[var(--muted-foreground)]">
            Not counted in your workspace
          </span>
        </div>
        <CohortDeptTabs groups={groups} activeId={activeId} onChange={onDeptChange} />
        {activeGroup ? (
          <CohortMagazineGrid
            group={activeGroup}
            onSelect={() => undefined}
            selectedId={null}
          />
        ) : null}
      </div>
    </section>
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
function CohortsHero({
  total,
  members,
  skills,
}: {
  total: number;
  members: number;
  skills: number;
}) {
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
    sub =
      "Each cohort earns its trust grade by shipping consistently. Promotions happen automatically.";
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
