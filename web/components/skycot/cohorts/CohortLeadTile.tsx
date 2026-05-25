"use client";

import type { CohortSummary } from "@/lib/api/cohorts";
import {
  cohortAvatarInitials,
  cohortDescription,
  cohortDisplayTitle,
  cohortKickerLabel,
  cohortPersonaTileMeta,
  trustBandLabel,
} from "./tile-meta";

export interface CohortLeadTileProps {
  cohort: CohortSummary;
  onSelect: (cohortId: string) => void;
  selected?: boolean;
}

/**
 * Big editorial "lead" tile for the D3 magazine layout. Audiowide title
 * with gradient accent on the trust-band word; ~2× the height of a
 * stacked tile; gradient-tinted background and "featured" pill badge.
 * Used as the first tile in each department block.
 */
export function CohortLeadTile({ cohort, onSelect, selected }: CohortLeadTileProps) {
  const initials = cohortAvatarInitials(cohort);
  const { plain, accent } = cohortDisplayTitle(cohort);
  const bandLabel = trustBandLabel(cohort.trust_grade_band);
  const kicker = cohortKickerLabel(cohort);
  const desc = cohortDescription(cohort);
  const persona = cohortPersonaTileMeta(cohort);
  return (
    <button
      type="button"
      data-testid={`cohort-card-${cohort.cohort_id}`}
      data-cohort-tile="lead"
      data-selected={selected ? "true" : "false"}
      onClick={() => onSelect(cohort.cohort_id)}
      aria-label={
        persona
          ? `Open cohort ${persona.name} ${persona.role}`
          : `Open cohort ${cohort.vertical} ${bandLabel}`
      }
      className="skycot-cohort-lead-tile"
    >
      <span className="skycot-cohort-lead-badge">
        {persona ? `${persona.level} worker` : "★ Lead cohort"}
      </span>
      <div className="skycot-cohort-lead-av" aria-hidden>
        {initials}
      </div>
      <div className="skycot-cohort-lead-kicker">{kicker}</div>
      <h3 className="skycot-cohort-lead-title">
        {plain}
        <strong>{accent}</strong>
      </h3>
      <p className="skycot-cohort-lead-desc">{desc}</p>
      <div className="skycot-cohort-lead-stats">
        {persona ? (
          <>
            <span className="skycot-cohort-lead-stat is-grad">
              Stack <strong>{persona.stack}</strong>
            </span>
            <span className="skycot-cohort-lead-stat">
              Great at <strong>{persona.greatAt[0]}</strong>
            </span>
            <span className="skycot-cohort-lead-stat">
              Escalates when <strong>{persona.escalatesWhen[0]}</strong>
            </span>
          </>
        ) : (
          <>
            <span className="skycot-cohort-lead-stat is-grad">
              Members <strong>{cohort.member_count}</strong>
            </span>
            <span className="skycot-cohort-lead-stat">
              Skills <strong>{cohort.skill_count}</strong>
            </span>
            <span className="skycot-cohort-lead-stat">
              Grade
              <strong>
                {" "}
                {cohort.trust_grade_min}
                {cohort.trust_grade_max > cohort.trust_grade_min
                  ? `–${cohort.trust_grade_max}`
                  : ""}
              </strong>
            </span>
          </>
        )}
      </div>
      <div className="skycot-cohort-lead-foot">
        <span className="skycot-cohort-lead-meta">cohort v1.0 · open details →</span>
      </div>
    </button>
  );
}
