import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { CohortSummary } from "@/lib/api/cohorts";
import { CohortStackedTile } from "./CohortStackedTile";

const COHORT: CohortSummary = {
  cohort_id: "sales::established",
  vertical: "sales",
  trust_grade_band: "established",
  trust_grade_min: 4,
  trust_grade_max: 6,
  member_count: 3,
  skill_count: 4,
};

describe("CohortStackedTile", () => {
  it("renders cohort summary fields and invokes onSelect on click", async () => {
    const onSelect = vi.fn();
    render(<CohortStackedTile cohort={COHORT} onSelect={onSelect} />);
    const tile = screen.getByTestId(`cohort-card-${COHORT.cohort_id}`);
    expect(tile.getAttribute("data-cohort-tile")).toBe("stacked");
    expect(screen.getByText(/ESTABLISHED COHORT/i)).toBeInTheDocument();
    expect(screen.getByText("Established")).toBeInTheDocument();
    expect(screen.getByText("sales")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("4")).toBeInTheDocument();
    await userEvent.click(tile);
    expect(onSelect).toHaveBeenCalledWith(COHORT.cohort_id);
  });
});
