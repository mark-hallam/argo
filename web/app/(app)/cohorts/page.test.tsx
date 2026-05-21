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

// Replace tile components so the page test only asserts page-shell
// behavior (dept-tab interaction, selected cohort propagation, drawer
// lifecycle). Each tile component has its own focused test file.
vi.mock("@/components/skycot/cohorts/CohortLeadTile", () => ({
  CohortLeadTile: makeTileMock("lead"),
}));
vi.mock("@/components/skycot/cohorts/CohortStackedTile", () => ({
  CohortStackedTile: makeTileMock("stacked"),
}));
vi.mock("@/components/skycot/cohorts/CohortCompactTile", () => ({
  CohortCompactTile: makeTileMock("compact"),
}));

function makeTileMock(variant: "lead" | "stacked" | "compact") {
  return function MockTile({
    cohort,
    onSelect,
    selected,
  }: {
    cohort: CohortSummary;
    onSelect: (id: string) => void;
    selected?: boolean;
  }) {
    return (
      <button
        type="button"
        data-testid={`cohort-card-${cohort.cohort_id}`}
        data-cohort-tile={variant}
        data-selected={selected ? "true" : "false"}
        onClick={() => onSelect(cohort.cohort_id)}
      >
        {cohort.vertical}
      </button>
    );
  };
}

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
    cohort_id: "support::developing",
    vertical: "support",
    trust_grade_band: "developing",
    trust_grade_min: 1,
    trust_grade_max: 3,
    member_count: 2,
    skill_count: 3,
  },
  {
    cohort_id: "sales::established",
    vertical: "sales",
    trust_grade_band: "established",
    trust_grade_min: 4,
    trust_grade_max: 6,
    member_count: 3,
    skill_count: 4,
  },
];

beforeEach(() => {
  useCohortsMock.mockReset();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("CohortsPage", () => {
  it("renders dept tabs and the magazine grid for the first dept by default", () => {
    useCohortsMock.mockReturnValue({
      data: { cohorts: SUMMARIES, source: "mock-offline" },
      isPending: false,
    });
    render(<CohortsPage />);
    expect(screen.getByTestId("cohort-grid")).toBeInTheDocument();
    expect(screen.getByTestId("cohorts-subtitle")).toHaveTextContent(/3 cohorts/i);
    // Default active dept is "support" (first in registry with data).
    expect(screen.getByTestId("cohort-dept-tab-support")).toHaveAttribute(
      "aria-selected",
      "true",
    );
    // Both support cohorts render; sales (different dept) does not.
    expect(screen.getByTestId("cohort-card-support::trusted")).toBeInTheDocument();
    expect(screen.getByTestId("cohort-card-support::developing")).toBeInTheDocument();
    expect(screen.queryByTestId("cohort-card-sales::established")).not.toBeInTheDocument();
  });

  it("opens the drawer when a tile is clicked", async () => {
    useCohortsMock.mockReturnValue({
      data: { cohorts: SUMMARIES, source: "mock-offline" },
      isPending: false,
    });
    render(<CohortsPage />);
    expect(screen.queryByTestId("drawer-open")).not.toBeInTheDocument();
    await userEvent.click(screen.getByTestId("cohort-card-support::trusted"));
    const drawer = screen.getByTestId("drawer-open");
    expect(drawer.getAttribute("data-cohort")).toBe("support::trusted");
  });

  it("switches the visible cohorts when a different dept tab is clicked", async () => {
    useCohortsMock.mockReturnValue({
      data: { cohorts: SUMMARIES, source: "mock-offline" },
      isPending: false,
    });
    render(<CohortsPage />);
    // Sales dept tab is present (Sales bucket contains "sales").
    const salesTab = screen.getByTestId("cohort-dept-tab-sales");
    await userEvent.click(salesTab);
    expect(salesTab).toHaveAttribute("aria-selected", "true");
    expect(screen.getByTestId("cohort-card-sales::established")).toBeInTheDocument();
    expect(screen.queryByTestId("cohort-card-support::trusted")).not.toBeInTheDocument();
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
