"use client";

import type { CohortSummary } from "@/lib/api/cohorts";
import { CohortCompactTile } from "./CohortCompactTile";
import { CohortLeadTile } from "./CohortLeadTile";
import { CohortStackedTile } from "./CohortStackedTile";
import type { DeptGroup } from "./dept-registry";

export interface CohortMagazineGridProps {
  group: DeptGroup;
  onSelect: (cohortId: string) => void;
  selectedId: string | null;
}

/**
 * D3 magazine asymmetric layout for a single department: 1 lead tile
 * (left, double-tall) + up to 2 stacked tiles (right column) + the
 * remaining cohorts in a 4-wide compact row. Adapts to fewer cohorts —
 * if a dept only has 3 cohorts, the lead + 2 stacked render and the
 * compact row collapses. Anything past the first 7 falls into the
 * compact row, which wraps.
 */
export function CohortMagazineGrid({
  group,
  onSelect,
  selectedId,
}: CohortMagazineGridProps) {
  const [lead, ...rest] = group.cohorts;
  const stacked: CohortSummary[] = rest.slice(0, 2);
  const compact: CohortSummary[] = rest.slice(2);
  if (!lead) return null;
  return (
    <div className="skycot-cohort-mag" data-testid={`cohort-mag-${group.dept.id}`}>
      <div className="skycot-cohort-mag-section-h">
        <h2>
          <strong>{group.dept.label}</strong> ·{" "}
          {group.cohorts.length}{" "}
          {group.cohorts.length === 1 ? "cohort" : "cohorts"}
        </h2>
        <div className="skycot-cohort-mag-section-meta">
          {group.dept.icon} {group.dept.label.toUpperCase()} · BUILT IN MELBOURNE
        </div>
      </div>
      <div className={stacked.length > 0 ? "skycot-cohort-mag-row1" : "skycot-cohort-mag-row1 is-solo"}>
        <CohortLeadTile
          cohort={lead}
          onSelect={onSelect}
          selected={lead.cohort_id === selectedId}
        />
        {stacked.length > 0 ? (
          <div className="skycot-cohort-mag-rightcol">
            {stacked.map((c) => (
              <CohortStackedTile
                key={c.cohort_id}
                cohort={c}
                onSelect={onSelect}
                selected={c.cohort_id === selectedId}
              />
            ))}
          </div>
        ) : null}
      </div>
      {compact.length > 0 ? (
        <div className="skycot-cohort-mag-row2">
          {compact.map((c) => (
            <CohortCompactTile
              key={c.cohort_id}
              cohort={c}
              onSelect={onSelect}
              selected={c.cohort_id === selectedId}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}
