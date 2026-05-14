import { render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CohortDetail, FetchCohortDetailResult } from "@/lib/api/cohorts";

type QueryShape = {
  data: FetchCohortDetailResult | undefined;
  isPending: boolean;
};

const useCohortDetailMock = vi.fn<() => QueryShape>();

vi.mock("@/lib/hooks/use-cohorts", () => ({
  useCohortDetail: () => useCohortDetailMock(),
}));

import { CohortDetailDrawer } from "./CohortDetailDrawer";

const DETAIL: CohortDetail = {
  cohort_id: "support::trusted",
  vertical: "support",
  trust_grade_band: "trusted",
  trust_grade_min: 7,
  trust_grade_max: 10,
  member_count: 2,
  skill_count: 2,
  members: [
    { worker_id: "wkr_alpha", trust_grade: 9, skill_ids: ["follow_up", "qualify"] },
    { worker_id: "wkr_bravo", trust_grade: 7, skill_ids: ["follow_up"] },
  ],
  skills: [
    {
      skill_name: "follow_up",
      version: "1.2.0",
      member_coverage_pct: 100.0,
      member_hits: 2,
    },
    {
      skill_name: "qualify",
      version: null,
      member_coverage_pct: 50.0,
      member_hits: 1,
    },
  ],
};

beforeEach(() => {
  useCohortDetailMock.mockReset();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("CohortDetailDrawer", () => {
  it("renders the coverage matrix with present + missing cells when data is loaded", () => {
    useCohortDetailMock.mockReturnValue({
      data: { detail: DETAIL, source: "mock-offline" },
      isPending: false,
    });
    render(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />);
    const matrix = screen.getByTestId("cohort-coverage-matrix");
    // 2 members × 2 skills = 4 cells; "follow_up" row covers both workers (present),
    // "qualify" row has 1 present + 1 missing.
    expect(within(matrix).getAllByTestId("cohort-cell-present").length).toBe(3);
    expect(within(matrix).getAllByTestId("cohort-cell-missing").length).toBe(1);
    // Version label rendered when present, omitted when null.
    expect(within(matrix).getByText("v1.2.0")).toBeInTheDocument();
  });

  it("renders nothing when cohortId is null (drawer closed)", () => {
    useCohortDetailMock.mockReturnValue({ data: undefined, isPending: false });
    const { container } = render(
      <CohortDetailDrawer cohortId={null} onClose={() => {}} />,
    );
    expect(container.firstChild).toBeNull();
  });
});
