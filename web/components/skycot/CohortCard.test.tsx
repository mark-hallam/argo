import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { CohortCard } from "./CohortCard";
import type { CohortSummary } from "@/lib/api/cohorts";

const COHORT: CohortSummary = {
  cohort_id: "support::trusted",
  vertical: "support",
  trust_grade_band: "trusted",
  trust_grade_min: 7,
  trust_grade_max: 10,
  member_count: 6,
  skill_count: 8,
};

describe("CohortCard", () => {
  it("renders vertical, band, member + skill counts and invokes onSelect", async () => {
    const onSelect = vi.fn();
    render(<CohortCard cohort={COHORT} onSelect={onSelect} />);
    expect(screen.getByText(/support/i)).toBeInTheDocument();
    expect(screen.getByText("Trusted")).toBeInTheDocument();
    expect(screen.getByText("6")).toBeInTheDocument();
    expect(screen.getByText("8")).toBeInTheDocument();
    await userEvent.click(screen.getByTestId(`cohort-card-${COHORT.cohort_id}`));
    expect(onSelect).toHaveBeenCalledWith(COHORT.cohort_id);
  });
});
