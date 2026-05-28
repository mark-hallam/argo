import type { Meta, StoryObj } from "@storybook/react";
import { SkillLearningLoopCard } from "./SkillLearningLoopCard";
import { sampleSkillTelemetry } from "@/lib/sample-skill-telemetry";

/**
 * Three variants the v1.0 demo Scene 4 needs to read at a glance:
 *
 * - "Fresh skill (0 runs)" — empty state, no fabricated history.
 * - "Promoted skill (B→A)" — the canonical learning-loop story.
 * - "High adoption" — 30-day sparkline trending up, 184 runs.
 *
 * Stories pull from `sample-skill-telemetry.ts` so the visual
 * contract here matches what ships in the skill-detail page.
 */

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
  name: "Promoted skill — B → A after 23 runs",
  args: {
    skillName: "approval.route",
    telemetry: sampleSkillTelemetry("approval.route"),
  },
};

export const HighAdoption: Story = {
  name: "High adoption — 184 runs across 11 workers",
  args: {
    skillName: "pricing.competitor_scrape",
    telemetry: sampleSkillTelemetry("pricing.competitor_scrape"),
  },
};
