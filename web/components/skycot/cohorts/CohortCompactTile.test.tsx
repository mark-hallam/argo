import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { CohortSummary } from "@/lib/api/cohorts";
import { CohortCompactTile } from "./CohortCompactTile";

const COHORT: CohortSummary = {
  cohort_id: "marketing::unknown",
  vertical: "marketing",
  trust_grade_band: "unknown",
  trust_grade_min: 0,
  trust_grade_max: 0,
  member_count: 1,
  skill_count: 1,
};

describe("CohortCompactTile", () => {
  it("renders cohort summary fields and invokes onSelect on click", async () => {
    const onSelect = vi.fn();
    render(<CohortCompactTile cohort={COHORT} onSelect={onSelect} />);
    const tile = screen.getByTestId(`cohort-card-${COHORT.cohort_id}`);
    expect(tile.getAttribute("data-cohort-tile")).toBe("compact");
    expect(screen.getByText(/PRE-TRUST COHORT/i)).toBeInTheDocument();
    expect(screen.getByText("Pre-trust")).toBeInTheDocument();
    expect(screen.getByText("marketing")).toBeInTheDocument();
    // Member + skill counts both render as <strong>1</strong>.
    expect(screen.getAllByText("1").length).toBeGreaterThanOrEqual(2);
    await userEvent.click(tile);
    expect(onSelect).toHaveBeenCalledWith(COHORT.cohort_id);
  });
});
