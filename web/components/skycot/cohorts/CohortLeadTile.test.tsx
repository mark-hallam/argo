import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { CohortSummary } from "@/lib/api/cohorts";
import { CohortLeadTile } from "./CohortLeadTile";

const COHORT: CohortSummary = {
  cohort_id: "support::trusted",
  vertical: "support",
  trust_grade_band: "trusted",
  trust_grade_min: 7,
  trust_grade_max: 10,
  member_count: 6,
  skill_count: 8,
};

const PETA: CohortSummary = {
  cohort_id: "project-ops::project-coordinator",
  vertical: "project-ops",
  trust_grade_band: "developing",
  trust_grade_min: 2,
  trust_grade_max: 2,
  member_count: 1,
  skill_count: 4,
};

describe("CohortLeadTile", () => {
  it("renders trust-band, member/skill counts, grade range, and invokes onSelect", async () => {
    const onSelect = vi.fn();
    render(<CohortLeadTile cohort={COHORT} onSelect={onSelect} />);
    // Lead tile carries the data-cohort-tile sentinel + cohort-card testid.
    const tile = screen.getByTestId(`cohort-card-${COHORT.cohort_id}`);
    expect(tile.getAttribute("data-cohort-tile")).toBe("lead");
    // Trust band rendered via JetBrains-Mono kicker.
    expect(screen.getByText(/TRUSTED COHORT/i)).toBeInTheDocument();
    // Member + skill counts.
    expect(screen.getByText("6")).toBeInTheDocument();
    expect(screen.getByText("8")).toBeInTheDocument();
    // Grade range when min < max.
    expect(screen.getByText(/7\s*–\s*10/)).toBeInTheDocument();
    await userEvent.click(tile);
    expect(onSelect).toHaveBeenCalledWith(COHORT.cohort_id);
  });

  it("shows only single grade when min == max", () => {
    const single = { ...COHORT, trust_grade_min: 9, trust_grade_max: 9 };
    render(<CohortLeadTile cohort={single} onSelect={() => undefined} />);
    // Single grade — no en-dash range present.
    expect(screen.queryByText(/9\s*–\s*9/)).not.toBeInTheDocument();
    // The grade strong element exists with just "9" (and a leading space).
    const tile = screen.getByTestId(`cohort-card-${single.cohort_id}`);
    expect(tile.textContent).toMatch(/Grade\s+9(?!\d)/);
  });

  it("reflects the selected prop on the tile button", () => {
    render(<CohortLeadTile cohort={COHORT} onSelect={() => undefined} selected />);
    const tile = screen.getByTestId(`cohort-card-${COHORT.cohort_id}`);
    expect(tile.getAttribute("data-selected")).toBe("true");
  });

  it("renders Peta's Project Coordinator persona when used as the lead tile", () => {
    render(<CohortLeadTile cohort={PETA} onSelect={() => undefined} />);
    expect(screen.getByText("L2 · Project Coordinator")).toBeInTheDocument();
    expect(screen.getByText("Peta")).toBeInTheDocument();
    expect(screen.getByText("Project Coordinator")).toBeInTheDocument();
    expect(
      screen.getByText("Coordinates work, action items, and standups across teams."),
    ).toBeInTheDocument();
    expect(screen.getByText("Linear · GitHub · Slack")).toBeInTheDocument();
    expect(screen.getByText("Action-item chase")).toBeInTheDocument();
    expect(screen.getByText("Slipping milestones")).toBeInTheDocument();
  });
});
