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
  recent_activity: [
    {
      seq: 99,
      kind: "skill.applied",
      action_type: "skill_apply",
      worker_id: "wkr_alpha",
      principal_id: "human:mark@acmeco.com",
      channel: "slack",
      outcome_status: "success",
      created_at_ms: Date.now() - 3 * 60_000,
      summary: "follow_up resolved ticket #4821",
    },
  ],
};

const EMPTY_COHORT_DETAIL: CohortDetail = {
  ...DETAIL,
  member_count: 0,
  skill_count: 0,
  members: [],
  skills: [],
  recent_activity: [],
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

  it("renders a loading skeleton while the query is pending", () => {
    useCohortDetailMock.mockReturnValue({ data: undefined, isPending: true });
    render(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />);
    expect(screen.getByTestId("cohort-detail-loading")).toBeInTheDocument();
  });

  it("lists cohort members with grade and skill count", () => {
    useCohortDetailMock.mockReturnValue({
      data: { detail: DETAIL, source: "mock-offline" },
      isPending: false,
    });
    render(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />);
    const rows = screen.getAllByTestId("cohort-detail-member-row");
    expect(rows.length).toBe(2);
    expect(rows[0]).toHaveAttribute("data-worker-id", "wkr_alpha");
    expect(within(rows[0]).getByText(/grade 9/)).toBeInTheDocument();
    expect(within(rows[0]).getByText(/2 skills/)).toBeInTheDocument();
    expect(within(rows[1]).getByText(/1 skill/)).toBeInTheDocument();
  });

  it("renders the recent activity feed when rows are present", () => {
    useCohortDetailMock.mockReturnValue({
      data: { detail: DETAIL, source: "mock-offline" },
      isPending: false,
    });
    render(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />);
    const rows = screen.getAllByTestId("cohort-detail-activity-row");
    expect(rows.length).toBe(1);
    expect(
      within(rows[0]).getByText("follow_up resolved ticket #4821"),
    ).toBeInTheDocument();
  });

  it("renders the no-members invite CTA when the cohort has zero members", () => {
    useCohortDetailMock.mockReturnValue({
      data: { detail: EMPTY_COHORT_DETAIL, source: "mock-offline" },
      isPending: false,
    });
    render(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />);
    expect(screen.getByTestId("cohort-detail-empty")).toBeInTheDocument();
    const cta = screen.getByTestId("cohort-detail-invite-cta");
    expect(cta).toHaveAttribute(
      "href",
      "/hire?cohort=support%3A%3Atrusted",
    );
  });

  it("falls through to the empty/invite panel when the gateway returns 404 (detail null)", () => {
    useCohortDetailMock.mockReturnValue({
      data: { detail: null, source: "gateway" },
      isPending: false,
    });
    render(<CohortDetailDrawer cohortId="ops::unknown" onClose={() => {}} />);
    expect(screen.getByTestId("cohort-detail-empty")).toBeInTheDocument();
    expect(screen.getByTestId("cohort-detail-invite-cta")).toBeInTheDocument();
  });
});
