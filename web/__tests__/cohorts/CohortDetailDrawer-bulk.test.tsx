import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { CohortDetail, FetchCohortDetailResult } from "@/lib/api/cohorts";
import type { SkillRow } from "@/lib/api/skills";

// `useCohortDetail` is mocked so we can drive the drawer with a fixed
// payload. `bulkAssignSkill` is mocked to verify the modal-confirm
// path posts the right { cohortId, workerIds, skillId } tuple.
type QueryShape = {
  data: FetchCohortDetailResult | undefined;
  isPending: boolean;
};

const useCohortDetailMock = vi.fn<() => QueryShape>();
const bulkAssignSkillMock = vi.fn();

vi.mock("@/lib/hooks/use-cohorts", () => ({
  useCohortDetail: () => useCohortDetailMock(),
}));

vi.mock("@/lib/api/cohorts", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/api/cohorts")>("@/lib/api/cohorts");
  return {
    ...actual,
    bulkAssignSkill: (...args: unknown[]) => bulkAssignSkillMock(...args),
  };
});

// Skill catalogue used by the modal — bypass the gateway fetch.
const SAMPLE_SKILLS: SkillRow[] = [
  {
    id: "skill-follow_up",
    name: "follow_up",
    version: "1.2.0",
    description: "Polite follow-up cadence.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
  {
    id: "skill-qualify",
    name: "qualify",
    version: "0.9.0",
    description: "BANT-style qualifier.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
];

vi.mock("@/lib/api/skills", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/api/skills")>("@/lib/api/skills");
  return {
    ...actual,
    fetchSkills: vi.fn(async () => ({ skills: SAMPLE_SKILLS, source: "mock-offline" })),
  };
});

import { CohortDetailDrawer } from "@/components/skycot/CohortDetailDrawer";

const DETAIL: CohortDetail = {
  cohort_id: "support::trusted",
  vertical: "support",
  trust_grade_band: "trusted",
  trust_grade_min: 7,
  trust_grade_max: 10,
  member_count: 2,
  skill_count: 1,
  members: [
    { worker_id: "wkr_alpha", trust_grade: 9, skill_ids: ["follow_up"] },
    { worker_id: "wkr_bravo", trust_grade: 7, skill_ids: [] },
  ],
  skills: [
    {
      skill_name: "follow_up",
      version: "1.2.0",
      member_coverage_pct: 50.0,
      member_hits: 1,
    },
  ],
  recent_activity: [],
};

function wrap(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
  return <QueryClientProvider client={client}>{ui}</QueryClientProvider>;
}

beforeEach(() => {
  useCohortDetailMock.mockReset();
  bulkAssignSkillMock.mockReset();
  useCohortDetailMock.mockReturnValue({
    data: { detail: DETAIL, source: "mock-offline" },
    isPending: false,
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("CohortDetailDrawer bulk-select + bulk-assign-skill", () => {
  it("renders a leading checkbox column on each member row", () => {
    render(wrap(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />));
    expect(screen.getByTestId("cohort-detail-select-all")).toBeInTheDocument();
    const rowCheckboxes = screen.getAllByTestId("cohort-detail-select-row");
    expect(rowCheckboxes.length).toBe(2);
  });

  it("hides the bulk-assign button until ≥1 row is selected", async () => {
    const user = userEvent.setup();
    render(wrap(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />));
    expect(screen.queryByTestId("cohort-detail-bulk-assign-button")).toBeNull();
    await user.click(screen.getAllByTestId("cohort-detail-select-row")[0]);
    expect(screen.getByTestId("cohort-detail-bulk-assign-button")).toBeInTheDocument();
  });

  it("header checkbox toggles selection across all members", async () => {
    const user = userEvent.setup();
    render(wrap(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />));
    await user.click(screen.getByTestId("cohort-detail-select-all"));
    const rows = screen.getAllByTestId("cohort-detail-select-row");
    expect(rows.every((el) => (el as HTMLInputElement).checked)).toBe(true);
    // Click the bulk button so the count reflects the full selection.
    expect(screen.getByTestId("cohort-detail-bulk-assign-button").textContent).toContain(
      "Assign skill to 2",
    );
    // Click again → all clear.
    await user.click(screen.getByTestId("cohort-detail-select-all"));
    expect(screen.queryByTestId("cohort-detail-bulk-assign-button")).toBeNull();
  });

  it("opens the bulk-assign modal and posts the selected workers + skill on confirm", async () => {
    bulkAssignSkillMock.mockResolvedValue({ assigned: 1, already_had: 0, errors: [] });
    const user = userEvent.setup();
    render(wrap(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />));
    await user.click(screen.getAllByTestId("cohort-detail-select-row")[1]);
    await user.click(screen.getByTestId("cohort-detail-bulk-assign-button"));

    // Modal opens; search filters to "qualify" only.
    const modal = await screen.findByTestId("bulk-assign-skill-modal");
    const search = within(modal).getByTestId("bulk-assign-skill-search");
    await user.type(search, "qual");
    const options = within(modal).getAllByTestId("bulk-assign-skill-option");
    expect(options.length).toBe(1);
    expect(options[0]).toHaveAttribute("data-skill-name", "qualify");

    // Pick + confirm.
    await user.click(options[0]);
    await user.click(within(modal).getByTestId("bulk-assign-skill-confirm"));

    await waitFor(() => {
      expect(bulkAssignSkillMock).toHaveBeenCalledWith(
        "support::trusted",
        ["wkr_bravo"],
        "skill-qualify",
      );
    });
  });

  it("renders the optimistic '✓ skill X added' badge for ~3s on each affected row", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    bulkAssignSkillMock.mockResolvedValue({ assigned: 2, already_had: 0, errors: [] });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(wrap(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />));
    await user.click(screen.getByTestId("cohort-detail-select-all"));
    await user.click(screen.getByTestId("cohort-detail-bulk-assign-button"));
    const modal = await screen.findByTestId("bulk-assign-skill-modal");
    const options = within(modal).getAllByTestId("bulk-assign-skill-option");
    await user.click(
      options.find((el) => el.getAttribute("data-skill-name") === "qualify") ?? options[0],
    );
    await user.click(within(modal).getByTestId("bulk-assign-skill-confirm"));

    const badges = await screen.findAllByTestId("cohort-detail-row-badge");
    expect(badges.length).toBe(2);
    expect(badges[0]).toHaveAttribute("data-skill-name", "qualify");

    // Advance past the badge timeout — badges drop off.
    act(() => {
      vi.advanceTimersByTime(3500);
    });
    await waitFor(() => {
      expect(screen.queryAllByTestId("cohort-detail-row-badge").length).toBe(0);
    });
    vi.useRealTimers();
  });
});
