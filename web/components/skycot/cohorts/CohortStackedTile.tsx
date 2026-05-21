"use client";

import type { CohortSummary } from "@/lib/api/cohorts";
import {
  cohortAvatarInitials,
  cohortDescription,
  cohortKickerLabel,
  trustBandLabel,
} from "./tile-meta";

export interface CohortStackedTileProps {
  cohort: CohortSummary;
  onSelect: (cohortId: string) => void;
  selected?: boolean;
}

/**
 * Medium "stacked" tile — half-width of the lead, sits in the right
 * column of row 1 of the D3 magazine grid. Inter title (not Audiowide)
 * to keep visual weight balanced against the lead's hero copy.
 */
export function CohortStackedTile({ cohort, onSelect, selected }: CohortStackedTileProps) {
  const initials = cohortAvatarInitials(cohort);
  const bandLabel = trustBandLabel(cohort.trust_grade_band);
  const kicker = cohortKickerLabel(cohort);
  const desc = cohortDescription(cohort);
  return (
    <button
      type="button"
      data-testid={`cohort-card-${cohort.cohort_id}`}
      data-cohort-tile="stacked"
      data-selected={selected ? "true" : "false"}
      onClick={() => onSelect(cohort.cohort_id)}
      aria-label={`Open cohort ${cohort.vertical} ${bandLabel}`}
      className="skycot-cohort-stacked-tile"
    >
      <div className="skycot-cohort-stacked-av" aria-hidden>
        {initials}
      </div>
      <div className="skycot-cohort-stacked-kicker">{kicker}</div>
      <h3 className="skycot-cohort-stacked-title">
        <span className="skycot-cohort-stacked-band">{bandLabel}</span>
        <span className="skycot-cohort-stacked-vertical">{cohort.vertical}</span>
      </h3>
      <p className="skycot-cohort-stacked-desc">{desc}</p>
      <div className="skycot-cohort-stacked-stats">
        <span>
          Members <strong>{cohort.member_count}</strong>
        </span>
        <span>
          Skills <strong>{cohort.skill_count}</strong>
        </span>
        <span>
          Grade
          <strong>
            {" "}
            {cohort.trust_grade_min}
            {cohort.trust_grade_max > cohort.trust_grade_min
              ? `–${cohort.trust_grade_max}`
              : ""}
          </strong>
        </span>
      </div>
    </button>
  );
}
