"use client";

import type { CohortSummary } from "@/lib/api/cohorts";
import {
  cohortAvatarInitials,
  cohortDescription,
  cohortKickerLabel,
  trustBandLabel,
} from "./tile-meta";

export interface CohortCompactTileProps {
  cohort: CohortSummary;
  onSelect: (cohortId: string) => void;
  selected?: boolean;
}

/**
 * Small "compact" tile — row 2 of the D3 magazine grid, four-up. Trim
 * padding and avatar size; description clamps to two lines so the row
 * stays visually even.
 */
export function CohortCompactTile({ cohort, onSelect, selected }: CohortCompactTileProps) {
  const initials = cohortAvatarInitials(cohort);
  const bandLabel = trustBandLabel(cohort.trust_grade_band);
  const kicker = cohortKickerLabel(cohort);
  const desc = cohortDescription(cohort);
  return (
    <button
      type="button"
      data-testid={`cohort-card-${cohort.cohort_id}`}
      data-cohort-tile="compact"
      data-selected={selected ? "true" : "false"}
      onClick={() => onSelect(cohort.cohort_id)}
      aria-label={`Open cohort ${cohort.vertical} ${bandLabel}`}
      className="skycot-cohort-compact-tile"
    >
      <div className="skycot-cohort-compact-av" aria-hidden>
        {initials}
      </div>
      <div className="skycot-cohort-compact-kicker">{kicker}</div>
      <h3 className="skycot-cohort-compact-title">
        <span className="skycot-cohort-compact-band">{bandLabel}</span>
        <span className="skycot-cohort-compact-vertical">{cohort.vertical}</span>
      </h3>
      <p className="skycot-cohort-compact-desc">{desc}</p>
      <div className="skycot-cohort-compact-stats">
        <span>
          <strong>{cohort.member_count}</strong> members
        </span>
        <span>
          <strong>{cohort.skill_count}</strong> skills
        </span>
      </div>
    </button>
  );
}
