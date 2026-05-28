import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SkillLearningLoopCard } from "./SkillLearningLoopCard";
import { sampleSkillTelemetry } from "@/lib/sample-skill-telemetry";

describe("SkillLearningLoopCard", () => {
  it("renders the three sub-blocks for a promoted skill", () => {
    const telemetry = sampleSkillTelemetry("approval.route");
    expect(telemetry).not.toBeNull();
    render(
      <SkillLearningLoopCard
        skillName="approval.route"
        telemetry={telemetry}
      />,
    );

    const card = screen.getByTestId("skill-learning-loop-card");
    expect(card).toHaveAttribute("data-state", "populated");
    expect(card).toHaveTextContent("Learning loop · Trust grade A");

    const subheader = screen.getByTestId("skill-learning-loop-subheader");
    expect(subheader).toHaveTextContent("Earned A on");
    expect(subheader).toHaveTextContent("23 successful runs");
    expect(subheader).toHaveTextContent("4 workers");

    const history = screen.getByTestId("skill-learning-loop-history");
    const events = within(history).getAllByRole("listitem");
    expect(events).toHaveLength(2);
    expect(events[0]).toHaveTextContent("Workspace promotion");
    expect(events[1]).toHaveTextContent("Marketplace promotion");

    const sparkline = screen.getByTestId("skill-learning-loop-sparkline");
    expect(within(sparkline).getByRole("img")).toBeInTheDocument();

    const runs = screen.getByTestId("skill-learning-loop-runs");
    const runRows = within(runs).getAllByRole("listitem");
    expect(runRows).toHaveLength(3);
    expect(runRows[0]).toHaveTextContent("Mariah");
    expect(runRows[0]).toHaveTextContent("approved");
  });

  it("renders the empty state when telemetry is null (no fabricated history)", () => {
    render(
      <SkillLearningLoopCard
        skillName="fresh.skill"
        telemetry={null}
      />,
    );

    const card = screen.getByTestId("skill-learning-loop-card");
    expect(card).toHaveAttribute("data-state", "empty");
    expect(card).toHaveTextContent("Learning loop · Trust grade B");

    const empty = screen.getByTestId("skill-learning-loop-empty");
    expect(empty).toHaveTextContent("No runs yet");
    expect(empty).toHaveTextContent("earns A through real workers");
    expect(empty).toHaveTextContent("fresh.skill");

    expect(screen.queryByTestId("skill-learning-loop-history")).toBeNull();
    expect(screen.queryByTestId("skill-learning-loop-sparkline")).toBeNull();
    expect(screen.queryByTestId("skill-learning-loop-runs")).toBeNull();
  });

  it("renders the high-adoption skill with 30-day sparkline + run-count subheader", () => {
    const telemetry = sampleSkillTelemetry("pricing.competitor_scrape");
    expect(telemetry).not.toBeNull();
    render(
      <SkillLearningLoopCard
        skillName="pricing.competitor_scrape"
        telemetry={telemetry}
      />,
    );

    const subheader = screen.getByTestId("skill-learning-loop-subheader");
    expect(subheader).toHaveTextContent("184 successful runs");
    expect(subheader).toHaveTextContent("11 workers");

    const sparkline = screen.getByTestId("skill-learning-loop-sparkline");
    const svg = within(sparkline).getByRole("img");
    expect(svg).toHaveAttribute(
      "aria-label",
      expect.stringContaining("pricing.competitor_scrape"),
    );

    const history = screen.getByTestId("skill-learning-loop-history");
    expect(history).toHaveTextContent("12 runs");
    expect(history).toHaveTextContent("47 runs");
  });

  it("respects a custom testId", () => {
    const telemetry = sampleSkillTelemetry("approval.route");
    render(
      <SkillLearningLoopCard
        skillName="approval.route"
        telemetry={telemetry}
        testId="custom-loop-card"
      />,
    );
    expect(screen.getByTestId("custom-loop-card")).toBeInTheDocument();
    expect(screen.queryByTestId("skill-learning-loop-card")).toBeNull();
  });
});
