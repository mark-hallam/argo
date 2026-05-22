import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { CohortDetail, FetchCohortDetailResult } from "@/lib/api/cohorts";
import { ApiError } from "@/lib/api/types";

// `useCohortDetail` is mocked so we can drive the drawer with a fixed
// payload. `disableSkill` is mocked so we can verify the DELETE wire-up
// without round-tripping to a gateway. The toast surface is mocked so
// we can assert exactly which tone fires per scenario.
type QueryShape = {
  data: FetchCohortDetailResult | undefined;
  isPending: boolean;
};

const useCohortDetailMock = vi.fn<() => QueryShape>();
const disableSkillMock = vi.hoisted(() => vi.fn());
const toastSuccess = vi.hoisted(() => vi.fn());
const toastError = vi.hoisted(() => vi.fn());

vi.mock("@/lib/hooks/use-cohorts", () => ({
  useCohortDetail: () => useCohortDetailMock(),
}));

vi.mock("@/lib/api/worker-skills", () => ({
  disableSkill: disableSkillMock,
}));

vi.mock("@/lib/ui/use-toast", () => ({
  useToast: () => ({
    success: toastSuccess,
    error: toastError,
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    promise: vi.fn(),
    dismiss: vi.fn(),
  }),
}));

import { CohortDetailDrawer } from "@/components/skycot/CohortDetailDrawer";

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
  recent_activity: [],
};

function wrap(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  const invalidateSpy = vi.spyOn(client, "invalidateQueries");
  return {
    client,
    invalidateSpy,
    ...render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>),
  };
}

beforeEach(() => {
  useCohortDetailMock.mockReset();
  disableSkillMock.mockReset();
  toastSuccess.mockReset();
  toastError.mockReset();
  useCohortDetailMock.mockReturnValue({
    data: { detail: DETAIL, source: "mock-offline" },
    isPending: false,
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("CohortDetailDrawer — per-chip skill removal", () => {
  it("renders an × button on every skill chip for every member", () => {
    wrap(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />);
    // alpha holds two skills, bravo holds one — total 3 chip × buttons.
    const removeButtons = screen.getAllByTestId("cohort-skill-chip-remove");
    expect(removeButtons.length).toBe(3);
  });

  it("opens the confirm modal naming the skill and worker when × is clicked", async () => {
    const user = userEvent.setup();
    wrap(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />);
    const rows = screen.getAllByTestId("cohort-detail-member-row");
    const alphaRow = rows[0]!;
    const removeFollowUp = within(alphaRow)
      .getAllByTestId("cohort-skill-chip-remove")
      .find((b) => b.getAttribute("data-skill-id") === "follow_up");
    expect(removeFollowUp).toBeTruthy();
    await user.click(removeFollowUp!);

    const modal = await screen.findByTestId("remove-skill-confirm-modal");
    const prompt = within(modal).getByTestId("remove-skill-confirm-prompt");
    expect(prompt.textContent).toContain("follow_up");
    expect(prompt.textContent).toContain("wkr_alpha");
  });

  it("DELETEs via disableSkill, hides the chip, refreshes the query, and toasts success", async () => {
    const user = userEvent.setup();
    disableSkillMock.mockResolvedValue({ id: "follow_up", source: "gateway" });
    const { invalidateSpy } = wrap(
      <CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />,
    );

    const alphaRow = screen.getAllByTestId("cohort-detail-member-row")[0]!;
    const removeBtn = within(alphaRow)
      .getAllByTestId("cohort-skill-chip-remove")
      .find((b) => b.getAttribute("data-skill-id") === "follow_up");
    await user.click(removeBtn!);

    await user.click(screen.getByTestId("remove-skill-confirm-submit"));

    await waitFor(() => {
      expect(disableSkillMock).toHaveBeenCalledWith("wkr_alpha", "follow_up");
    });
    await waitFor(() => {
      expect(toastSuccess).toHaveBeenCalled();
    });
    // Modal closes.
    await waitFor(() => {
      expect(screen.queryByTestId("remove-skill-confirm-modal")).toBeNull();
    });
    // Cohort detail invalidation fires so the activity feed refreshes.
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["cohort", "support::trusted"],
    });
    // alpha's row now reports the one remaining skill chip.
    const alphaRowAfter = screen.getAllByTestId("cohort-detail-member-row")[0]!;
    const chipsAfter = within(alphaRowAfter).queryAllByTestId(
      "cohort-skill-chip",
    );
    expect(chipsAfter.length).toBe(1);
    expect(chipsAfter[0]).toHaveAttribute("data-skill-id", "qualify");
  });

  it("treats a 404 as idempotent success: chip disappears, no error toast", async () => {
    const user = userEvent.setup();
    disableSkillMock.mockRejectedValue(new ApiError(404, "not found", "NOT_FOUND"));
    wrap(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />);

    const alphaRow = screen.getAllByTestId("cohort-detail-member-row")[0]!;
    const removeBtn = within(alphaRow)
      .getAllByTestId("cohort-skill-chip-remove")
      .find((b) => b.getAttribute("data-skill-id") === "qualify");
    await user.click(removeBtn!);
    await user.click(screen.getByTestId("remove-skill-confirm-submit"));

    await waitFor(() => {
      expect(screen.queryByTestId("remove-skill-confirm-modal")).toBeNull();
    });
    expect(toastError).not.toHaveBeenCalled();
    const alphaRowAfter = screen.getAllByTestId("cohort-detail-member-row")[0]!;
    const chipsAfter = within(alphaRowAfter).queryAllByTestId(
      "cohort-skill-chip",
    );
    expect(chipsAfter.find((c) => c.getAttribute("data-skill-id") === "qualify"))
      .toBeUndefined();
  });

  it("surfaces the cross-tenant permission error on 403", async () => {
    const user = userEvent.setup();
    disableSkillMock.mockRejectedValue(new ApiError(403, "forbidden", "FORBIDDEN"));
    wrap(<CohortDetailDrawer cohortId="support::trusted" onClose={() => {}} />);

    const alphaRow = screen.getAllByTestId("cohort-detail-member-row")[0]!;
    const removeBtn = within(alphaRow)
      .getAllByTestId("cohort-skill-chip-remove")
      .find((b) => b.getAttribute("data-skill-id") === "follow_up");
    await user.click(removeBtn!);
    await user.click(screen.getByTestId("remove-skill-confirm-submit"));

    await waitFor(() => {
      expect(toastError).toHaveBeenCalledWith(
        "You don't have permission to modify that worker.",
      );
    });
    // Modal stays open with inline error.
    expect(screen.getByTestId("remove-skill-confirm-modal")).toBeInTheDocument();
    expect(screen.getByTestId("remove-skill-confirm-error")).toBeInTheDocument();
  });
});
