import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SkillLearningLoopCard } from "./SkillLearningLoopCard";
import type { SkillTelemetry } from "@/lib/api/skills";

/**
 * These tests pin that the card renders REAL telemetry (from the
 * `/api/v1/skills/<id>/telemetry` endpoint) and falls back to an honest
 * empty state — never a fabricated fixture. The `sample-skill-telemetry.ts`
 * fixture was DELETED; this suite builds its own minimal real-shaped
 * bundles inline so it cannot accidentally depend on fixture constants.
 */

/** A real-shaped populated bundle (mirrors the gateway envelope). */
const POPULATED: SkillTelemetry = {
  skill_id: "11111111-1111-1111-1111-111111111111",
  skill_name: "pricing.competitor_scrape",
  current_grade: "published_verified",
  run_count: 3,
  distinct_workers: 2,
  eval_history: [
    { version: "v1", pass_rate: 0.86, previous_pass_rate: null, decision: "no_baseline", ran_at: "2026-05-18T09:00:00.000Z" },
    { version: "v2", pass_rate: 0.91, previous_pass_rate: 0.86, decision: "stable", ran_at: "2026-05-24T09:00:00.000Z" },
    { version: "v3", pass_rate: 0.72, previous_pass_rate: 0.91, decision: "regressed", ran_at: "2026-05-29T09:00:00.000Z" },
  ],
  adoption_trend: [0, 0, 1, 1, 1, 2, 2, 2, 2, 2],
  recent_runs: [
    { version: "v3", pass_rate: 0.72, decision: "regressed", ran_at: "2026-05-29T09:00:00.000Z" },
    { version: "v2", pass_rate: 0.91, decision: "stable", ran_at: "2026-05-24T09:00:00.000Z" },
    { version: "v1", pass_rate: 0.86, decision: "no_baseline", ran_at: "2026-05-18T09:00:00.000Z" },
  ],
};

describe("SkillLearningLoopCard", () => {
  it("renders the three sub-blocks from REAL telemetry", () => {
    render(
      <SkillLearningLoopCard skillName="pricing.competitor_scrape" telemetry={POPULATED} />,
    );

    const card = screen.getByTestId("skill-learning-loop-card");
    expect(card).toHaveAttribute("data-state", "populated");
    expect(card).toHaveTextContent("Learning loop · Trust grade A");

    const subheader = screen.getByTestId("skill-learning-loop-subheader");
    // Real counts, straight from the bundle — not fixture constants.
    expect(subheader).toHaveTextContent("3 eval re-runs");
    expect(subheader).toHaveTextContent("2 workers");

    const history = screen.getByTestId("skill-learning-loop-history");
    const events = within(history).getAllByRole("listitem");
    expect(events).toHaveLength(3);
    expect(events[0]).toHaveTextContent("baseline");
    expect(events[2]).toHaveTextContent("regressed");
    expect(events[1]).toHaveTextContent("91% pass");

    const sparkline = screen.getByTestId("skill-learning-loop-sparkline");
    expect(within(sparkline).getByRole("img")).toBeInTheDocument();

    const runs = screen.getByTestId("skill-learning-loop-runs");
    const runRows = within(runs).getAllByRole("listitem");
    expect(runRows).toHaveLength(3);
    // Newest-first; the row shows the real version + pass rate, NOT a
    // fabricated worker name or latency.
    expect(runRows[0]).toHaveTextContent("v3");
    expect(runRows[0]).toHaveTextContent("72% pass");
    expect(runs).not.toHaveTextContent("ms");
    expect(runs).not.toHaveTextContent("Mariah");
  });

  it("renders the empty state when telemetry is null (no fabricated history)", () => {
    render(<SkillLearningLoopCard skillName="fresh.skill" telemetry={null} />);

    const card = screen.getByTestId("skill-learning-loop-card");
    expect(card).toHaveAttribute("data-state", "empty");

    const empty = screen.getByTestId("skill-learning-loop-empty");
    expect(empty).toHaveTextContent("No runs yet");
    expect(empty).toHaveTextContent("earns its trust grade through real workers");
    expect(empty).toHaveTextContent("fresh.skill");

    expect(screen.queryByTestId("skill-learning-loop-history")).toBeNull();
    expect(screen.queryByTestId("skill-learning-loop-sparkline")).toBeNull();
    expect(screen.queryByTestId("skill-learning-loop-runs")).toBeNull();
  });

  it("renders the empty state when a real bundle has zero runs", () => {
    const empty: SkillTelemetry = {
      skill_id: "22222222-2222-2222-2222-222222222222",
      skill_name: "lead.enrich",
      current_grade: "learned",
      run_count: 0,
      distinct_workers: 0,
      eval_history: [],
      adoption_trend: [0, 0, 0, 0, 0],
      recent_runs: [],
    };
    render(<SkillLearningLoopCard skillName="lead.enrich" telemetry={empty} />);
    const card = screen.getByTestId("skill-learning-loop-card");
    expect(card).toHaveAttribute("data-state", "empty");
    expect(screen.getByTestId("skill-learning-loop-empty")).toHaveTextContent("No runs yet");
  });

  it("respects a custom testId", () => {
    render(
      <SkillLearningLoopCard
        skillName="pricing.competitor_scrape"
        telemetry={POPULATED}
        testId="custom-loop-card"
      />,
    );
    expect(screen.getByTestId("custom-loop-card")).toBeInTheDocument();
    expect(screen.queryByTestId("skill-learning-loop-card")).toBeNull();
  });
});
