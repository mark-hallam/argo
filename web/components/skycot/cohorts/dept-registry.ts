import type { CohortSummary } from "@/lib/api/cohorts";

/**
 * Department taxonomy for the D3 magazine layout. Each cohort vertical
 * maps to a department; tabs above the magazine grid filter by dept.
 * Order here drives tab display order. Verticals not in the registry
 * fall back to an "Other" department so the page never silently drops
 * data.
 */

export interface Department {
  id: string;
  label: string;
  icon: string;
  verticals: readonly string[];
}

export const DEPARTMENTS: readonly Department[] = [
  { id: "support", label: "Support", icon: "🎧", verticals: ["support"] },
  {
    id: "finance",
    label: "Finance & Ops",
    icon: "💰",
    verticals: ["finance", "ops", "operations", "accounting"],
  },
  { id: "sales", label: "Sales", icon: "📊", verticals: ["sales", "marketplace"] },
  {
    id: "project-ops",
    label: "Project & Ops",
    icon: "📋",
    verticals: ["project", "projects", "pm", "project-ops"],
  },
  {
    id: "people",
    label: "People & HR",
    icon: "👥",
    verticals: ["people", "hr", "recruiting"],
  },
  {
    id: "it-devex",
    label: "IT & Dev-ex",
    icon: "🔧",
    verticals: ["it", "engineering", "devex", "infrastructure"],
  },
  {
    id: "marketing",
    label: "Marketing",
    icon: "📣",
    verticals: ["marketing"],
  },
  { id: "legal", label: "Legal", icon: "⚖", verticals: ["legal", "compliance"] },
];

const OTHER_DEPT: Department = {
  id: "other",
  label: "Other",
  icon: "◐",
  verticals: [],
};

export function deptForVertical(vertical: string): Department {
  const v = vertical.toLowerCase();
  for (const d of DEPARTMENTS) {
    if (d.verticals.includes(v)) return d;
  }
  return OTHER_DEPT;
}

export interface DeptGroup {
  dept: Department;
  cohorts: CohortSummary[];
}

/**
 * Bucket cohorts into departments preserving the DEPARTMENTS ordering.
 * Empty departments are dropped — the tab strip only shows depts that
 * have at least one cohort, so the page reflects real data.
 */
export function groupCohortsByDepartment(cohorts: readonly CohortSummary[]): DeptGroup[] {
  const buckets = new Map<string, DeptGroup>();
  for (const cohort of cohorts) {
    const dept = deptForVertical(cohort.vertical);
    const existing = buckets.get(dept.id);
    if (existing) {
      existing.cohorts.push(cohort);
    } else {
      buckets.set(dept.id, { dept, cohorts: [cohort] });
    }
  }
  const ordered: DeptGroup[] = [];
  for (const dept of DEPARTMENTS) {
    const group = buckets.get(dept.id);
    if (group) ordered.push(group);
  }
  const other = buckets.get(OTHER_DEPT.id);
  if (other) ordered.push(other);
  return ordered;
}
