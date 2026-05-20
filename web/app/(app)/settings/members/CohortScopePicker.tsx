"use client";

import { useCohorts } from "@/lib/hooks/use-cohorts";
import { cohortLabel } from "./cohort-format";
import { roleOverridesCohortScope, type MemberRole } from "./members-data";

interface CohortScopePickerProps {
  role: MemberRole;
  /** `null` → "All cohorts" toggle is active. */
  value: string[] | null;
  onChange: (next: string[] | null) => void;
  /** Hide the helper-text + override banner (Invite modal sets this true). */
  compactHelper?: boolean;
}

/**
 * Multi-select chip picker for cohort scope, shared by the Edit
 * Member modal and the Invite Member modal.
 *
 * - Toggle row: "All cohorts" vs "Specific cohorts".
 * - When role overrides scope (owner / admin) the picker is forced
 *   to "All cohorts" with a small banner explaining why; the
 *   onChange callback will only ever receive `null` for those roles.
 * - When "Specific cohorts" is active the chip grid lists every
 *   workspace cohort; click toggles membership.
 */
export function CohortScopePicker({
  role,
  value,
  onChange,
  compactHelper,
}: CohortScopePickerProps) {
  const cohortsQuery = useCohorts();
  const cohorts = cohortsQuery.data?.cohorts ?? [];
  const overrides = roleOverridesCohortScope(role);
  const allMode = overrides || value === null;
  const selected = overrides ? null : value;

  const toggleCohort = (cohortId: string) => {
    if (overrides) return;
    const current = selected ?? [];
    const next = current.includes(cohortId)
      ? current.filter((id) => id !== cohortId)
      : [...current, cohortId];
    onChange(next);
  };

  return (
    <div data-testid="cohort-scope-picker">
      {overrides ? (
        <div className="scope-override-note" data-testid="cohort-scope-override-note">
          {role === "owner" ? "Owners" : "Admins"} always see every cohort. Switch
          the role to <strong>User</strong> or <strong>Guest</strong> to enable
          cohort scoping.
        </div>
      ) : null}

      <div className="scope-toggle-row">
        <button
          type="button"
          className={`scope-toggle-pill ${allMode ? "selected" : ""}`}
          onClick={() => onChange(null)}
          disabled={overrides}
          data-testid="cohort-scope-toggle-all"
        >
          All cohorts
          <span className="scope-toggle-hint">Default for owner + admin.</span>
        </button>
        <button
          type="button"
          className={`scope-toggle-pill ${!allMode ? "selected" : ""}`}
          onClick={() => onChange(selected ?? [])}
          disabled={overrides}
          data-testid="cohort-scope-toggle-specific"
        >
          Specific cohorts
          <span className="scope-toggle-hint">Limit to the cohorts you pick.</span>
        </button>
      </div>

      {!allMode ? (
        <div className="scope-cohort-grid" data-testid="cohort-scope-grid">
          {cohorts.length === 0 ? (
            <div className="scope-cohort-empty">
              {cohortsQuery.isPending ? "Loading cohorts…" : "No cohorts in this workspace yet."}
            </div>
          ) : (
            cohorts.map((cohort) => {
              const isSelected = (selected ?? []).includes(cohort.cohort_id);
              return (
                <button
                  key={cohort.cohort_id}
                  type="button"
                  className={`scope-cohort-chip ${isSelected ? "selected" : ""}`}
                  onClick={() => toggleCohort(cohort.cohort_id)}
                  data-testid={`cohort-scope-chip-${cohort.cohort_id}`}
                  aria-pressed={isSelected}
                >
                  {isSelected ? "✓ " : ""}
                  {cohortLabel(cohort)}
                </button>
              );
            })
          )}
        </div>
      ) : null}

      {compactHelper ? null : (
        <p className="scope-helper">
          Member can only see workers, workflows, and audit entries belonging
          to selected cohorts.
        </p>
      )}
    </div>
  );
}
