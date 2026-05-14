import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CohortSummary, FetchCohortsResult } from "@/lib/api/cohorts";

type QueryShape = {
  data: FetchCohortsResult | undefined;
  isPending: boolean;
};

const useCohortsMock = vi.fn<() => QueryShape>();

vi.mock("@/lib/hooks/use-cohorts", () => ({
  useCohorts: () => useCohortsMock(),
  // Detail-drawer hook is stubbed out — page-level tests don't render
  // the matrix.
  useCohortDetail: () => ({ data: undefined, isPending: false }),
}));

// Replace child components so the page test only asserts page-shell
// behavior. Each test file for the children covers their own surface.
vi.mock("@/components/skycot/CohortCard", () => ({
  CohortCard: ({
    cohort,
    onSelect,
    selected,
  }: {
    cohort: CohortSummary;
    onSelect: (id: string) => void;
    selected?: boolean;
  }) => (
    <button
      type="button"
      data-testid={`cohort-card-${cohort.cohort_id}`}
      data-selected={selected ? "true" : "false"}
      onClick={() => onSelect(cohort.cohort_id)}
    >
      {cohort.vertical}
    </button>
  ),
}));

vi.mock("@/components/skycot/CohortDetailDrawer", () => ({
  CohortDetailDrawer: ({
    cohortId,
    onClose,
  }: {
    cohortId: string | null;
    onClose: () => void;
  }) =>
    cohortId === null ? null : (
      <div data-testid="drawer-open" data-cohort={cohortId}>
        <button type="button" data-testid="drawer-close" onClick={onClose}>
          close
        </button>
      </div>
    ),
}));

import CohortsPage from "./page";

const SUMMARIES: CohortSummary[] = [
  {
    cohort_id: "support::trusted",
    vertical: "support",
    trust_grade_band: "trusted",
    trust_grade_min: 7,
    trust_grade_max: 10,
    member_count: 4,
    skill_count: 6,
  },
  {
    cohort_id: "sales::developing",
    vertical: "sales",
    trust_grade_band: "developing",
    trust_grade_min: 1,
    trust_grade_max: 3,
    member_count: 2,
    skill_count: 3,
  },
];

beforeEach(() => {
  useCohortsMock.mockReset();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("CohortsPage", () => {
  it("renders all cohort cards and opens the drawer when a card is clicked", async () => {
    useCohortsMock.mockReturnValue({
      data: { cohorts: SUMMARIES, source: "mock-offline" },
      isPending: false,
    });
    render(<CohortsPage />);
    expect(screen.getByTestId("cohort-grid")).toBeInTheDocument();
    expect(screen.getByText(/2 cohorts/i)).toBeInTheDocument();
    expect(screen.getAllByTestId(/^cohort-card-/).length).toBe(2);
    expect(screen.queryByTestId("drawer-open")).not.toBeInTheDocument();
    await userEvent.click(screen.getByTestId("cohort-card-support::trusted"));
    const drawer = screen.getByTestId("drawer-open");
    expect(drawer.getAttribute("data-cohort")).toBe("support::trusted");
  });

  it("renders the empty state when the fleet has no cohorts", () => {
    useCohortsMock.mockReturnValue({
      data: { cohorts: [], source: "mock-offline" },
      isPending: false,
    });
    render(<CohortsPage />);
    expect(screen.getByTestId("cohorts-empty")).toBeInTheDocument();
    expect(screen.queryByTestId("cohort-grid")).not.toBeInTheDocument();
  });
});
