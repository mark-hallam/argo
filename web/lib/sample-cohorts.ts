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

import type {
  CohortActivityRow,
  CohortDetail,
  CohortSummary,
  SkillCoverage,
} from "@/lib/api/cohorts";

export const SAMPLE_COHORT_SUMMARIES: CohortSummary[] = [
  {
    cohort_id: "support::trusted",
    vertical: "support",
    trust_grade_band: "trusted",
    trust_grade_min: 7,
    trust_grade_max: 10,
    member_count: 5,
    skill_count: 7,
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
    member_count: 1,
    skill_count: 3,
  },
  {
    cohort_id: "project-ops::project-coordinator",
    vertical: "project-ops",
    trust_grade_band: "developing",
    trust_grade_min: 2,
    trust_grade_max: 2,
    member_count: 1,
    skill_count: 4,
  },
  {
    cohort_id: "project-ops::status-reporter",
    vertical: "project-ops",
    trust_grade_band: "developing",
    trust_grade_min: 3,
    trust_grade_max: 3,
    member_count: 1,
    skill_count: 5,
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

const PROJECT_COORDINATOR_SKILLS: SkillCoverage[] = [
  {
    skill_name: "linear_read",
    version: "1.0.0",
    member_coverage_pct: 100.0,
    member_hits: 1,
  },
  {
    skill_name: "github_read",
    version: "1.0.0",
    member_coverage_pct: 100.0,
    member_hits: 1,
  },
  {
    skill_name: "slack_dm",
    version: "1.0.0",
    member_coverage_pct: 100.0,
    member_hits: 1,
  },
  {
    skill_name: "action_item_extract",
    version: "1.0.0",
    member_coverage_pct: 100.0,
    member_hits: 1,
  },
];

const STATUS_REPORTER_SKILLS: SkillCoverage[] = [
  {
    skill_name: "status_report_draft",
    version: "1.0.0",
    member_coverage_pct: 100.0,
    member_hits: 1,
  },
  {
    skill_name: "raid_log_maintain",
    version: "1.0.0",
    member_coverage_pct: 100.0,
    member_hits: 1,
  },
  {
    skill_name: "milestone_track",
    version: "1.0.0",
    member_coverage_pct: 100.0,
    member_hits: 1,
  },
  {
    skill_name: "exec_summary_draft",
    version: "1.0.0",
    member_coverage_pct: 100.0,
    member_hits: 1,
  },
  {
    skill_name: "stakeholder_outbound_draft",
    version: "1.0.0",
    member_coverage_pct: 100.0,
    member_hits: 1,
  },
];

const SUPPORT_TRUSTED_SKILLS: SkillCoverage[] = [
  {
    skill_name: "triage_intent",
    version: "1.4.0",
    member_coverage_pct: 100.0,
    member_hits: 5,
  },
  {
    skill_name: "summarize_thread",
    version: "1.2.0",
    member_coverage_pct: 80.0,
    member_hits: 4,
  },
  {
    skill_name: "escalate_to_human",
    version: "0.9.1",
    member_coverage_pct: 80.0,
    member_hits: 4,
  },
  {
    skill_name: "refund_lookup",
    version: "2.0.0",
    member_coverage_pct: 60.0,
    member_hits: 3,
  },
  {
    skill_name: "tag_followup",
    version: "1.0.0",
    member_coverage_pct: 40.0,
    member_hits: 2,
  },
  {
    skill_name: "schedule_callback",
    version: null,
    member_coverage_pct: 20.0,
    member_hits: 1,
  },
  {
    skill_name: "draft_apology",
    version: "1.1.0",
    member_coverage_pct: 20.0,
    member_hits: 1,
  },
];

const MARKETPLACE_DEVELOPING_SKILLS: SkillCoverage[] = [
  {
    skill_name: "list_inventory",
    version: "1.0.0",
    member_coverage_pct: 100.0,
    member_hits: 1,
  },
  {
    skill_name: "reprice_items",
    version: "1.2.0",
    member_coverage_pct: 100.0,
    member_hits: 1,
  },
  {
    skill_name: "counter_offers",
    version: "0.9.0",
    member_coverage_pct: 100.0,
    member_hits: 1,
  },
];

const NOW_MS = 1_716_336_000_000; // 2024-05-22T00:00:00Z, anchored so snapshot diffs stay stable.

const SUPPORT_TRUSTED_ACTIVITY: CohortActivityRow[] = [
  {
    seq: 4201,
    kind: "skill.applied",
    action_type: "skill_apply",
    worker_id: "018f67d0-0000-7000-8000-000000000002",
    principal_id: "human:mark@acmeco.com",
    channel: "slack",
    outcome_status: "success",
    created_at_ms: NOW_MS - 5 * 60_000,
    summary: "refund_lookup → resolved escalated ticket #4821",
  },
  {
    seq: 4198,
    kind: "policy.escalated",
    action_type: "escalate",
    worker_id: "018f67d0-0000-7000-8000-000000000003",
    principal_id: "human:ops-on-call",
    channel: "email",
    outcome_status: "approved",
    created_at_ms: NOW_MS - 22 * 60_000,
    summary: "draft_apology held for human review (sentiment < -0.7)",
  },
  {
    seq: 4193,
    kind: "skill.promoted",
    action_type: "trust_promotion",
    worker_id: "018f67d0-0000-7000-8000-000000000006",
    principal_id: "system:trust_engine",
    channel: null,
    outcome_status: "success",
    created_at_ms: NOW_MS - 60 * 60_000,
    summary: "grade 6 → 7 after 14-day clean streak",
  },
  {
    seq: 4188,
    kind: "approval.completed",
    action_type: "approval",
    worker_id: "018f67d0-0000-7000-8000-000000000004",
    principal_id: "human:mark@acmeco.com",
    channel: "web",
    outcome_status: "approved",
    created_at_ms: NOW_MS - 3 * 60 * 60_000,
    summary: "approved $480 refund (Mariah upsell, repeat customer)",
  },
];

const MARKETPLACE_DEVELOPING_ACTIVITY: CohortActivityRow[] = [
  {
    seq: 4090,
    kind: "skill.applied",
    action_type: "skill_apply",
    worker_id: "018f67d0-0000-7000-8000-000000000001",
    principal_id: null,
    channel: "internal",
    outcome_status: "success",
    created_at_ms: NOW_MS - 9 * 60_000,
    summary: "list_inventory ran on 142 SKUs (vintage_apparel)",
  },
];

const PROJECT_COORDINATOR_ACTIVITY: CohortActivityRow[] = [
  {
    seq: 4082,
    kind: "summary.drafted",
    action_type: "draft_summary",
    worker_id: "wkr_01J0PETA",
    principal_id: "human:ops-lead",
    channel: "slack",
    outcome_status: "pending_approval",
    created_at_ms: NOW_MS - 36 * 60_000,
    summary: "standup digest drafted from Linear, GitHub, and Slack updates",
  },
];

const STATUS_REPORTER_ACTIVITY: CohortActivityRow[] = [
  {
    seq: 4078,
    kind: "raid.updated",
    action_type: "raid_log",
    worker_id: "wkr_01J0PATRICK",
    principal_id: "system:status-reporter",
    channel: "internal",
    outcome_status: "success",
    created_at_ms: NOW_MS - 44 * 60_000,
    summary: "RAID log refreshed; weekly executive summary remains draft-only",
  },
];

export const SAMPLE_COHORT_DETAIL: CohortDetail = {
  cohort_id: "support::trusted",
  vertical: "support",
  trust_grade_band: "trusted",
  trust_grade_min: 7,
  trust_grade_max: 10,
  member_count: 5,
  skill_count: 7,
  members: [
    {
      worker_id: "018f67d0-0000-7000-8000-000000000002",
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
      worker_id: "018f67d0-0000-7000-8000-000000000003",
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
      worker_id: "018f67d0-0000-7000-8000-000000000006",
      trust_grade: 7,
      skill_ids: ["triage_intent", "summarize_thread", "escalate_to_human", "refund_lookup"],
    },
    {
      worker_id: "018f67d0-0000-7000-8000-000000000004",
      trust_grade: 7,
      skill_ids: ["triage_intent", "summarize_thread", "tag_followup"],
    },
    {
      worker_id: "018f67d0-0000-7000-8000-000000000005",
      trust_grade: 7,
      skill_ids: ["triage_intent", "escalate_to_human"],
    },
  ],
  skills: SUPPORT_TRUSTED_SKILLS,
  recent_activity: SUPPORT_TRUSTED_ACTIVITY,
};

export const SAMPLE_MARKETPLACE_COHORT_DETAIL: CohortDetail = {
  cohort_id: "marketplace::developing",
  vertical: "marketplace",
  trust_grade_band: "developing",
  trust_grade_min: 1,
  trust_grade_max: 3,
  member_count: 1,
  skill_count: 3,
  members: [
    {
      worker_id: "018f67d0-0000-7000-8000-000000000001",
      trust_grade: 2,
      skill_ids: ["list_inventory", "reprice_items", "counter_offers"],
    },
  ],
  skills: MARKETPLACE_DEVELOPING_SKILLS,
  recent_activity: MARKETPLACE_DEVELOPING_ACTIVITY,
};

export const SAMPLE_PROJECT_COORDINATOR_COHORT_DETAIL: CohortDetail = {
  cohort_id: "project-ops::project-coordinator",
  vertical: "project-ops",
  trust_grade_band: "developing",
  trust_grade_min: 2,
  trust_grade_max: 2,
  member_count: 1,
  skill_count: 4,
  members: [
    {
      worker_id: "wkr_01J0PETA",
      trust_grade: 2,
      skill_ids: ["linear_read", "github_read", "slack_dm", "action_item_extract"],
    },
  ],
  skills: PROJECT_COORDINATOR_SKILLS,
  recent_activity: PROJECT_COORDINATOR_ACTIVITY,
};

export const SAMPLE_STATUS_REPORTER_COHORT_DETAIL: CohortDetail = {
  cohort_id: "project-ops::status-reporter",
  vertical: "project-ops",
  trust_grade_band: "developing",
  trust_grade_min: 3,
  trust_grade_max: 3,
  member_count: 1,
  skill_count: 5,
  members: [
    {
      worker_id: "wkr_01J0PATRICK",
      trust_grade: 3,
      skill_ids: [
        "status_report_draft",
        "raid_log_maintain",
        "milestone_track",
        "exec_summary_draft",
        "stakeholder_outbound_draft",
      ],
    },
  ],
  skills: STATUS_REPORTER_SKILLS,
  recent_activity: STATUS_REPORTER_ACTIVITY,
};

export const SAMPLE_COHORT_DETAILS: Record<string, CohortDetail> = {
  "support::trusted": SAMPLE_COHORT_DETAIL,
  "marketplace::developing": SAMPLE_MARKETPLACE_COHORT_DETAIL,
  "project-ops::project-coordinator": SAMPLE_PROJECT_COORDINATOR_COHORT_DETAIL,
  "project-ops::status-reporter": SAMPLE_STATUS_REPORTER_COHORT_DETAIL,
};
