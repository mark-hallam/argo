"use client";

import type { DeptGroup } from "./dept-registry";

export interface CohortDeptTabsProps {
  groups: readonly DeptGroup[];
  activeId: string;
  onChange: (deptId: string) => void;
}

/**
 * D3 dept-tab strip — one chip per department with at least one cohort.
 * Active chip gets the brand gradient on the icon swatch and a soft
 * gradient halo. Tabs filter the magazine grid below.
 */
export function CohortDeptTabs({ groups, activeId, onChange }: CohortDeptTabsProps) {
  return (
    <div className="skycot-cohort-tabs" role="tablist" data-testid="cohort-dept-tabs">
      {groups.map((g) => {
        const isOn = g.dept.id === activeId;
        const count = g.cohorts.length;
        return (
          <button
            key={g.dept.id}
            type="button"
            role="tab"
            aria-selected={isOn}
            data-testid={`cohort-dept-tab-${g.dept.id}`}
            data-on={isOn ? "true" : "false"}
            className={
              isOn ? "skycot-cohort-tab is-on" : "skycot-cohort-tab"
            }
            onClick={() => onChange(g.dept.id)}
          >
            <span className="skycot-cohort-tab-ico" aria-hidden>
              {g.dept.icon}
            </span>
            <span className="skycot-cohort-tab-name">{g.dept.label}</span>
            <span className="skycot-cohort-tab-ct">
              {count} {count === 1 ? "COHORT" : "COHORTS"}
            </span>
          </button>
        );
      })}
    </div>
  );
}
