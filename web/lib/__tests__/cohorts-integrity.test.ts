import { describe, expect, it } from "vitest";
import { SAMPLE_COHORT_DETAILS, SAMPLE_COHORT_SUMMARIES } from "@/lib/sample-cohorts";
import { SAMPLE_TEAM } from "@/lib/sample-team";

describe("sample cohorts data integrity", () => {
  const teamIds = new Set(SAMPLE_TEAM.map((w) => w.id));

  it("every cohort member references a worker in SAMPLE_TEAM", () => {
    const unknown: Array<{ cohortId: string; workerId: string }> = [];
    for (const [cohortId, detail] of Object.entries(SAMPLE_COHORT_DETAILS)) {
      for (const member of detail.members) {
        if (!teamIds.has(member.worker_id)) {
          unknown.push({ cohortId, workerId: member.worker_id });
        }
      }
    }
    expect(unknown).toEqual([]);
  });

  it("Mariah appears in her own marketplace cohort", () => {
    const marketplace = SAMPLE_COHORT_DETAILS["marketplace::developing"];
    expect(marketplace).toBeDefined();
    const memberIds = marketplace!.members.map((m) => m.worker_id);
    expect(memberIds).toContain("wkr_01J0MARIAH");
  });

  it("each cohort detail's member_count matches its members array length", () => {
    for (const [cohortId, detail] of Object.entries(SAMPLE_COHORT_DETAILS)) {
      expect(detail.members.length, `cohort ${cohortId} member count drift`).toBe(
        detail.member_count,
      );
    }
  });

  it("each cohort detail's skill_count matches its skills array length", () => {
    for (const [cohortId, detail] of Object.entries(SAMPLE_COHORT_DETAILS)) {
      expect(detail.skills.length, `cohort ${cohortId} skill count drift`).toBe(
        detail.skill_count,
      );
    }
  });

  it("each detail's summary fields agree with the matching SAMPLE_COHORT_SUMMARIES row", () => {
    const summaryById = new Map(SAMPLE_COHORT_SUMMARIES.map((s) => [s.cohort_id, s]));
    for (const [cohortId, detail] of Object.entries(SAMPLE_COHORT_DETAILS)) {
      const summary = summaryById.get(cohortId);
      expect(summary, `summary for ${cohortId}`).toBeDefined();
      expect(detail.member_count).toBe(summary!.member_count);
      expect(detail.skill_count).toBe(summary!.skill_count);
      expect(detail.vertical).toBe(summary!.vertical);
      expect(detail.trust_grade_band).toBe(summary!.trust_grade_band);
    }
  });
});
