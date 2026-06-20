import { describe, expect, it } from "vitest";
import type { CohortSummary } from "@/lib/api/cohorts";
import { DEPARTMENTS, deptForVertical, groupCohortsByDepartment } from "./dept-registry";

const SEED_COHORT_VERTICALS = [
  "marketing",
  "marketplace",
  "project-coordination",
  "recruiting",
  "support",
] as const;

const PACK_MANIFEST_VERTICALS = [
  "ap-specialist",
  "customer-support",
  "deal-desk",
  "finance-ar",
  "marketing-intern",
  "pitch-agent",
  "project-coordinator",
  "recruiter",
  "sales-sdr",
  "status-reporter",
  "support-tier1",
  "vendor-manager",
] as const;

// The GTM teammate verticals under verticals/gtm-*, mapped to the
// Marketing or Growth department in the cohort taxonomy.
const GTM_MARKETING_VERTICALS = [
  "gtm-paid-acquisition",
  "gtm-seo",
  "gtm-lifecycle-email",
  "gtm-brand-creative",
] as const;

const GTM_GROWTH_VERTICALS = [
  "gtm-growth-lead",
  "gtm-voice-of-customer",
  "gtm-growth-analytics",
  "gtm-revops-admin",
  "gtm-customer-success",
] as const;

function makeCohort(vertical: string): CohortSummary {
  return {
    cohort_id: `${vertical}::unknown`,
    vertical,
    trust_grade_band: "unknown",
    trust_grade_min: 0,
    trust_grade_max: 0,
    member_count: 1,
    skill_count: 0,
  };
}

describe("deptForVertical", () => {
  it("maps every demo-seed gateway vertical to a non-other department", () => {
    for (const v of SEED_COHORT_VERTICALS) {
      expect(deptForVertical(v).id, `seed vertical "${v}" should not bucket to other`).not.toBe(
        "other",
      );
    }
  });

  it("maps every pack-manifest vertical id to a non-other department", () => {
    for (const v of PACK_MANIFEST_VERTICALS) {
      expect(deptForVertical(v).id, `pack vertical "${v}" should not bucket to other`).not.toBe(
        "other",
      );
    }
  });

  it("routes the Peta (project-coordination) cohort to the Project & Ops tab", () => {
    expect(deptForVertical("project-coordination").id).toBe("project-ops");
  });

  it("routes the Patrick (status-reporter) cohort to the Project & Ops tab", () => {
    expect(deptForVertical("status-reporter").id).toBe("project-ops");
  });

  it("routes the GTM marketing teammates to the Marketing tab", () => {
    for (const v of GTM_MARKETING_VERTICALS) {
      expect(deptForVertical(v).id, `gtm vertical "${v}" should bucket to marketing`).toBe(
        "marketing",
      );
    }
  });

  it("routes the GTM growth teammates to the Growth tab", () => {
    for (const v of GTM_GROWTH_VERTICALS) {
      expect(deptForVertical(v).id, `gtm vertical "${v}" should bucket to growth`).toBe("growth");
    }
  });

  it("is case-insensitive", () => {
    expect(deptForVertical("PROJECT-COORDINATION").id).toBe("project-ops");
    expect(deptForVertical("Support").id).toBe("support");
  });

  it("falls back to other for an unknown vertical", () => {
    expect(deptForVertical("not-a-real-vertical").id).toBe("other");
  });
});

describe("groupCohortsByDepartment", () => {
  it("buckets the live demo-seed cohort set with no fall-through to other", () => {
    const groups = groupCohortsByDepartment(SEED_COHORT_VERTICALS.map(makeCohort));
    const ids = groups.map((g) => g.dept.id);
    expect(ids).not.toContain("other");
    expect(ids).toContain("project-ops");
  });

  it("preserves DEPARTMENTS declaration order in the returned groups", () => {
    const verticals = ["marketing", "support", "project-coordination", "recruiting"];
    const groups = groupCohortsByDepartment(verticals.map(makeCohort));
    const order = groups.map((g) => g.dept.id);
    const declared = DEPARTMENTS.map((d) => d.id);
    const indexed = order.map((id) => declared.indexOf(id));
    const sorted = [...indexed].sort((a, b) => a - b);
    expect(indexed).toEqual(sorted);
  });
});
