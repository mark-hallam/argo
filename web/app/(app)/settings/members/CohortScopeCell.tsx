"use client";

import { useCohorts } from "@/lib/hooks/use-cohorts";
import { cohortLabelFromId } from "./cohort-format";
import { roleOverridesCohortScope, type MemberRole } from "./members-data";

const VISIBLE_CHIP_LIMIT = 3;

interface CohortScopeCellProps {
  role: MemberRole;
  /** Member's scope: `null` → all cohorts. */
  cohortScope: string[] | null;
}

/**
 * Renders the "Cohort scope" column for one member row. Owner +
 * admin always render as "All cohorts" regardless of their stored
 * scope array, mirroring backend behavior where those roles bypass
 * cohort-filtering capability checks. The first 3 cohorts render as
 * chips; any remainder collapse into a "+N more" badge with a
 * native tooltip listing the rest.
 */
export function CohortScopeCell({ role, cohortScope }: CohortScopeCellProps) {
  const cohortsQuery = useCohorts();
  const cohorts = cohortsQuery.data?.cohorts ?? [];

  if (roleOverridesCohortScope(role) || cohortScope === null || cohortScope.length === 0) {
    return (
      <div className="member-scope-cell" data-testid="member-scope-all">
        <span className="member-scope-chip member-scope-chip-all">All cohorts</span>
      </div>
    );
  }

  const visible = cohortScope.slice(0, VISIBLE_CHIP_LIMIT);
  const overflow = cohortScope.slice(VISIBLE_CHIP_LIMIT);

  return (
    <div className="member-scope-cell" data-testid="member-scope-list">
      {visible.map((id) => (
        <span key={id} className="member-scope-chip">
          {cohortLabelFromId(id, cohorts)}
        </span>
      ))}
      {overflow.length > 0 ? (
        <span
          className="member-scope-more"
          title={overflow.map((id) => cohortLabelFromId(id, cohorts)).join(", ")}
        >
          +{overflow.length} more
        </span>
      ) : null}
    </div>
  );
}
