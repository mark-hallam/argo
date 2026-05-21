import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CohortMagazineSkeleton } from "./CohortMagazineSkeleton";

describe("CohortMagazineSkeleton", () => {
  it("renders 7 placeholders in the D3 lead/stacked/compact shape", () => {
    render(<CohortMagazineSkeleton />);
    // 1 lead + 2 stacked + 4 compact = 7 placeholders, matching the
    // locked /cohorts magazine layout so the grid is reserved before
    // real data hydrates.
    expect(screen.getAllByTestId("cohort-skel-lead")).toHaveLength(1);
    expect(screen.getAllByTestId("cohort-skel-stacked")).toHaveLength(2);
    expect(screen.getAllByTestId("cohort-skel-compact")).toHaveLength(4);
    // Wrapper carries the polite live-region role for assistive tech.
    const wrapper = screen.getByTestId("cohort-mag-skeleton");
    expect(wrapper).toHaveAttribute("role", "status");
    expect(wrapper).toHaveAttribute("aria-live", "polite");
  });
});
