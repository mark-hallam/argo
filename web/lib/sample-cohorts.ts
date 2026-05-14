/**
 * Mock cohort dataset for `/cohorts`. Returned by `fetchCohorts()` /
 * `fetchCohortDetail()` when the gateway returns 404 (typical in dev
 * with no `argo gateway` running) or when the auth-bypass flag is on.
 *
 * A **cohort** is a group of workers with the same `vertical` and a
 * coarse `trust_grade_band`. Five archetypes loosely mirror the
 * `sample-team` roster so the demo feels coherent if both surfaces
 * are running side-by-side.
 */

import type { CohortDetail, CohortSummary, SkillCoverage } from "@/lib/api/cohorts";

export const SAMPLE_COHORT_SUMMARIES: CohortSummary[] = [
  {
    cohort_id: "support::trusted",
    vertical: "support",
    trust_grade_band: "trusted",
    trust_grade_min: 7,
    trust_grade_max: 10,
    member_count: 6,
    skill_count: 8,
  },
  {
    cohort_id: "support::developing",
    vertical: "support",
    trust_grade_band: "developing",
    trust_grade_min: 1,
    trust_grade_max: 3,
    member_count: 4,
    skill_count: 5,
  },
  {
    cohort_id: "sales::established",
    vertical: "sales",
    trust_grade_band: "established",
    trust_grade_min: 4,
    trust_grade_max: 6,
    member_count: 3,
    skill_count: 4,
  },
  {
    cohort_id: "marketplace::developing",
    vertical: "marketplace",
    trust_grade_band: "developing",
    trust_grade_min: 1,
    trust_grade_max: 3,
    member_count: 2,
    skill_count: 3,
  },
  {
    cohort_id: "marketing::unknown",
    vertical: "marketing",
    trust_grade_band: "unknown",
    trust_grade_min: 0,
    trust_grade_max: 0,
    member_count: 1,
    skill_count: 1,
  },
];

const SUPPORT_TRUSTED_SKILLS: SkillCoverage[] = [
  {
    skill_name: "triage_intent",
    version: "1.4.0",
    member_coverage_pct: 100.0,
    member_hits: 6,
  },
  {
    skill_name: "summarize_thread",
    version: "1.2.0",
    member_coverage_pct: 83.3,
    member_hits: 5,
  },
  {
    skill_name: "escalate_to_human",
    version: "0.9.1",
    member_coverage_pct: 83.3,
    member_hits: 5,
  },
  {
    skill_name: "refund_lookup",
    version: "2.0.0",
    member_coverage_pct: 66.7,
    member_hits: 4,
  },
  {
    skill_name: "tag_followup",
    version: "1.0.0",
    member_coverage_pct: 50.0,
    member_hits: 3,
  },
  {
    skill_name: "schedule_callback",
    version: null,
    member_coverage_pct: 33.3,
    member_hits: 2,
  },
  {
    skill_name: "draft_apology",
    version: "1.1.0",
    member_coverage_pct: 33.3,
    member_hits: 2,
  },
  {
    skill_name: "compliance_check",
    version: "0.4.0",
    member_coverage_pct: 16.7,
    member_hits: 1,
  },
];

export const SAMPLE_COHORT_DETAIL: CohortDetail = {
  cohort_id: "support::trusted",
  vertical: "support",
  trust_grade_band: "trusted",
  trust_grade_min: 7,
  trust_grade_max: 10,
  member_count: 6,
  skill_count: 8,
  members: [
    {
      worker_id: "wkr_01J0MAYA",
      trust_grade: 9,
      skill_ids: [
        "triage_intent",
        "summarize_thread",
        "escalate_to_human",
        "refund_lookup",
        "tag_followup",
        "schedule_callback",
        "draft_apology",
        "compliance_check",
      ],
    },
    {
      worker_id: "wkr_01J0PRIYA",
      trust_grade: 8,
      skill_ids: [
        "triage_intent",
        "summarize_thread",
        "escalate_to_human",
        "refund_lookup",
        "tag_followup",
      ],
    },
    {
      worker_id: "wkr_01J0JAMAL",
      trust_grade: 8,
      skill_ids: [
        "triage_intent",
        "summarize_thread",
        "escalate_to_human",
        "refund_lookup",
        "schedule_callback",
        "draft_apology",
      ],
    },
    {
      worker_id: "wkr_01J0RIO",
      trust_grade: 7,
      skill_ids: ["triage_intent", "summarize_thread", "escalate_to_human", "refund_lookup"],
    },
    {
      worker_id: "wkr_01J0NORI",
      trust_grade: 7,
      skill_ids: ["triage_intent", "summarize_thread", "tag_followup"],
    },
    {
      worker_id: "wkr_01J0SAM",
      trust_grade: 7,
      skill_ids: ["triage_intent", "escalate_to_human"],
    },
  ],
  skills: SUPPORT_TRUSTED_SKILLS,
};

export const SAMPLE_COHORT_DETAILS: Record<string, CohortDetail> = {
  "support::trusted": SAMPLE_COHORT_DETAIL,
};
