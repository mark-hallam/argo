"use client";

import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { cn } from "@/lib/utils";
import type { CohortSummary } from "@/lib/api/cohorts";

const BAND_COPY: Record<string, string> = {
  unknown: "Pre-trust",
  developing: "Developing",
  established: "Established",
  trusted: "Trusted",
  principal: "Principal",
};

const BAND_TONE: Record<string, string> = {
  unknown: "bg-[var(--muted)] text-[var(--muted-foreground)]",
  developing: "bg-amber-100 text-amber-900",
  established: "bg-sky-100 text-sky-900",
  trusted: "bg-emerald-100 text-emerald-900",
  principal: "bg-violet-100 text-violet-900",
};

export interface CohortCardProps {
  cohort: CohortSummary;
  onSelect: (cohortId: string) => void;
  selected?: boolean;
}

export function CohortCard({ cohort, onSelect, selected }: CohortCardProps) {
  const bandLabel = BAND_COPY[cohort.trust_grade_band] ?? cohort.trust_grade_band;
  const bandTone = BAND_TONE[cohort.trust_grade_band] ?? BAND_TONE.unknown;
  return (
    <Card
      hoverable
      selected={selected}
      data-testid={`cohort-card-${cohort.cohort_id}`}
      role="button"
      tabIndex={0}
      onClick={() => onSelect(cohort.cohort_id)}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSelect(cohort.cohort_id);
        }
      }}
      aria-label={`Open cohort ${cohort.vertical} ${bandLabel}`}
    >
      <CardHeader>
        <div className="flex items-start justify-between gap-2">
          <CardTitle className="capitalize">{cohort.vertical}</CardTitle>
          <span
            className={cn("rounded-full px-2 py-0.5 text-[11px] font-medium", bandTone)}
            data-testid={`cohort-band-${cohort.cohort_id}`}
          >
            {bandLabel}
          </span>
        </div>
      </CardHeader>
      <CardBody>
        <dl className="grid grid-cols-2 gap-3 text-sm">
          <div>
            <dt className="text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
              Members
            </dt>
            <dd className="text-xl font-semibold text-[var(--foreground)]">
              {cohort.member_count}
            </dd>
          </div>
          <div>
            <dt className="text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
              Skills
            </dt>
            <dd className="text-xl font-semibold text-[var(--foreground)]">
              {cohort.skill_count}
            </dd>
          </div>
        </dl>
        <p className="mt-3 text-xs text-[var(--muted-foreground)]">
          Grade {cohort.trust_grade_min}
          {cohort.trust_grade_max > cohort.trust_grade_min
            ? `–${cohort.trust_grade_max}`
            : ""}
        </p>
      </CardBody>
    </Card>
  );
}
