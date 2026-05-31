import type { Meta, StoryObj } from "@storybook/react";
import { SkillLearningLoopCard } from "./SkillLearningLoopCard";
import type { SkillTelemetry } from "@/lib/api/skills";

/**
 * Three variants the v1.0 demo Scene 4 needs to read at a glance:
 *
 * - "Fresh skill (0 runs)" — empty state, no fabricated history.
 * - "Promoted skill" — eval re-runs trending up then a regression.
 * - "High adoption" — adoption sparkline trending up.
 *
 * The args below are REAL-SHAPED telemetry bundles (the same envelope
 * the `/api/v1/skills/<id>/telemetry` endpoint returns). The
 * `sample-skill-telemetry.ts` fixture was DELETED — these story args are
 * illustrative display data for Storybook only, never wired into the
 * shipped page (which reads the live gateway).
 */

const PROMOTED: SkillTelemetry = {
  skill_id: "stories-approval-route",
  skill_name: "approval.route",
  current_grade: "published_verified",
  run_count: 3,
  distinct_workers: 4,
  eval_history: [
    { version: "v1", pass_rate: 0.84, previous_pass_rate: null, decision: "no_baseline", ran_at: "2026-05-18T09:00:00.000Z" },
    { version: "v2", pass_rate: 0.9, previous_pass_rate: 0.84, decision: "stable", ran_at: "2026-05-24T09:00:00.000Z" },
    { version: "v3", pass_rate: 0.94, previous_pass_rate: 0.9, decision: "stable", ran_at: "2026-05-29T09:00:00.000Z" },
  ],
  adoption_trend: [0, 1, 1, 2, 2, 3, 3, 4, 4, 4],
  recent_runs: [
    { version: "v3", pass_rate: 0.94, decision: "stable", ran_at: "2026-05-29T09:00:00.000Z" },
    { version: "v2", pass_rate: 0.9, decision: "stable", ran_at: "2026-05-24T09:00:00.000Z" },
    { version: "v1", pass_rate: 0.84, decision: "no_baseline", ran_at: "2026-05-18T09:00:00.000Z" },
  ],
};

const HIGH_ADOPTION: SkillTelemetry = {
  skill_id: "stories-pricing-scrape",
  skill_name: "pricing.competitor_scrape",
  current_grade: "published_verified",
  run_count: 3,
  distinct_workers: 11,
  eval_history: [
    { version: "v1", pass_rate: 0.88, previous_pass_rate: null, decision: "no_baseline", ran_at: "2026-05-17T09:00:00.000Z" },
    { version: "v2", pass_rate: 0.9, previous_pass_rate: 0.88, decision: "stable", ran_at: "2026-05-23T09:00:00.000Z" },
    { version: "v3", pass_rate: 0.72, previous_pass_rate: 0.9, decision: "regressed", ran_at: "2026-05-29T09:00:00.000Z" },
  ],
  adoption_trend: [3, 4, 5, 6, 7, 8, 9, 10, 11, 11],
  recent_runs: [
    { version: "v3", pass_rate: 0.72, decision: "regressed", ran_at: "2026-05-29T09:00:00.000Z" },
    { version: "v2", pass_rate: 0.9, decision: "stable", ran_at: "2026-05-23T09:00:00.000Z" },
    { version: "v1", pass_rate: 0.88, decision: "no_baseline", ran_at: "2026-05-17T09:00:00.000Z" },
  ],
};

const meta: Meta<typeof SkillLearningLoopCard> = {
  title: "Skycot/Skills/SkillLearningLoopCard",
  component: SkillLearningLoopCard,
  parameters: { layout: "padded" },
};

export default meta;

type Story = StoryObj<typeof SkillLearningLoopCard>;

export const FreshSkill: Story = {
  name: "Fresh skill — 0 runs (empty state)",
  args: {
    skillName: "marketing.brief.summarize",
    telemetry: null,
  },
};

export const PromotedSkill: Story = {
  name: "Promoted skill — eval re-runs trending up",
  args: {
    skillName: "approval.route",
    telemetry: PROMOTED,
  },
};

export const HighAdoption: Story = {
  name: "High adoption — 11 workers, latest run regressed",
  args: {
    skillName: "pricing.competitor_scrape",
    telemetry: HIGH_ADOPTION,
  },
};
