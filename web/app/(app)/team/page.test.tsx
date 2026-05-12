import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Worker } from "@/lib/types/worker";

const replaceMock = vi.fn();
let searchParams = new URLSearchParams();

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    replace: replaceMock,
    push: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  usePathname: () => "/team",
  useSearchParams: () => searchParams,
}));

type FetchTeamResultShape = { workers: Worker[]; source: "gateway" | "mock-offline" | "mock-bypass" };
type QueryShape = {
  data: FetchTeamResultShape | undefined;
  isPending: boolean;
};

const useTeamMock = vi.fn<() => QueryShape>();

vi.mock("@/lib/hooks/use-team", () => ({
  useTeam: () => useTeamMock(),
}));

// Replace TeammateCard with a thin stub. The card has its own
// test file (TeammateCard.test.tsx); page-level tests assert the
// page's filtering / routing / empty-state logic and just need a
// stable testid per worker. Avoids dragging QueryClientProvider +
// router push into the page-test render tree.
vi.mock("@/components/skycot/TeammateCard", () => ({
  TeammateCard: ({ worker }: { worker: Worker }) => (
    <div data-testid={`teammate-${worker.id}`}>{worker.name}</div>
  ),
}));

import TeamPage from "./page";

const SAMPLE_WORKERS: Worker[] = [
  {
    id: "wkr_alpha",
    name: "Alpha",
    role: { tagline: "drafts replies", vertical: "support" },
    status: "working",
    lastActiveAt: "2026-05-12T10:00:00.000Z",
  },
  {
    id: "wkr_bravo",
    name: "Bravo",
    role: { tagline: "screens candidates", vertical: "recruiting" },
    status: "waiting",
    lastActiveAt: "2026-05-12T11:00:00.000Z",
  },
  {
    id: "wkr_charlie",
    name: "Charlie",
    role: { tagline: "watches the on-call queue", vertical: "ops" },
    status: "idle",
    lastActiveAt: "2026-05-12T09:00:00.000Z",
  },
];

function setSearchParams(qs: string) {
  searchParams = new URLSearchParams(qs);
}

beforeEach(() => {
  setSearchParams("");
  replaceMock.mockClear();
  useTeamMock.mockReset();
  useTeamMock.mockReturnValue({
    data: { workers: SAMPLE_WORKERS, source: "mock-offline" },
    isPending: false,
  });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("TeamPage", () => {
  it("renders the header with title, subtitle, status filter, and Hire CTA", () => {
    render(<TeamPage />);
    expect(screen.getByRole("heading", { level: 1, name: /team/i })).toBeInTheDocument();
    expect(screen.getByTestId("team-subtitle")).toHaveTextContent(
      `${SAMPLE_WORKERS.length} teammates working`,
    );
    expect(screen.getByTestId("team-status-filter")).toBeInTheDocument();
    expect(screen.getByTestId("team-hire-cta")).toHaveAttribute("href", "/hire");
  });

  it("renders a skeleton card for every worker in the grid", () => {
    render(<TeamPage />);
    const grid = screen.getByTestId("teammate-grid");
    for (const worker of SAMPLE_WORKERS) {
      expect(within(grid).getByTestId(`teammate-${worker.id}`)).toBeInTheDocument();
    }
  });

  it("renders the loading grid while the query is pending", () => {
    useTeamMock.mockReturnValue({ data: undefined, isPending: true });
    render(<TeamPage />);
    expect(screen.getByTestId("teammate-loading-0")).toBeInTheDocument();
    expect(screen.getByTestId("teammate-loading-5")).toBeInTheDocument();
    expect(screen.queryByTestId("team-empty-state")).not.toBeInTheDocument();
  });

  it("renders the first-hire empty state when the roster has zero workers", () => {
    useTeamMock.mockReturnValue({
      data: { workers: [], source: "mock-offline" },
      isPending: false,
    });
    render(<TeamPage />);
    const empty = screen.getByTestId("team-empty-state");
    expect(within(empty).getByText(/hire your first ai teammate\./i)).toBeInTheDocument();
    expect(
      within(empty).getByText(/pick a starting point — you can adjust everything/i),
    ).toBeInTheDocument();
    expect(within(empty).getByTestId("team-empty-state-tiles")).toBeInTheDocument();
    expect(within(empty).getByTestId("team-empty-state-describe-own")).toHaveAttribute(
      "href",
      "/hire",
    );
    expect(screen.queryByTestId("team-subtitle")).not.toBeInTheDocument();
  });

  it("filters the grid client-side when ?q= matches a worker name", () => {
    setSearchParams("q=alpha");
    render(<TeamPage />);
    expect(screen.getByTestId("teammate-wkr_alpha")).toBeInTheDocument();
    expect(screen.queryByTestId("teammate-wkr_bravo")).not.toBeInTheDocument();
    expect(screen.queryByTestId("teammate-wkr_charlie")).not.toBeInTheDocument();
  });

  it("filters the grid client-side when ?q= matches a role tagline", () => {
    setSearchParams("q=on-call");
    render(<TeamPage />);
    expect(screen.getByTestId("teammate-wkr_charlie")).toBeInTheDocument();
    expect(screen.queryByTestId("teammate-wkr_alpha")).not.toBeInTheDocument();
  });

  it("shows the no-search-matches state with a Clear-all CTA when query has no hits", () => {
    setSearchParams("q=zzz-nothing");
    render(<TeamPage />);
    const empty = screen.getByTestId("team-no-search-matches");
    expect(within(empty).getByText(/No matching teammates/)).toBeInTheDocument();
    expect(within(empty).getByText(/zzz-nothing/)).toBeInTheDocument();
    expect(within(empty).getByTestId("team-no-matches-clear-all")).toBeInTheDocument();
  });

  it("filters the grid by ?status= and shows the no-status-matches state when empty", () => {
    setSearchParams("status=paused");
    render(<TeamPage />);
    expect(screen.getByTestId("team-no-status-matches")).toBeInTheDocument();
    expect(screen.queryByTestId("teammate-wkr_alpha")).not.toBeInTheDocument();
  });

  it("updates the URL after a debounce when the user types in the search input", async () => {
    const user = userEvent.setup();
    render(<TeamPage />);
    await user.type(screen.getByTestId("team-search-input"), "a");
    await waitFor(
      () => {
        expect(replaceMock).toHaveBeenCalled();
        const lastCall = replaceMock.mock.calls.at(-1);
        expect(lastCall?.[0]).toContain("q=a");
      },
      { timeout: 1000 },
    );
  });

  it("updates the URL when the user picks a status filter", async () => {
    const user = userEvent.setup();
    render(<TeamPage />);
    await user.selectOptions(screen.getByTestId("team-status-filter"), "working");
    expect(replaceMock).toHaveBeenCalled();
    const lastCall = replaceMock.mock.calls.at(-1);
    expect(lastCall?.[0]).toContain("status=working");
  });

  // --- E3-d additions: chip rail + filter dot + filtered subtitle + clear flows ---

  it("renders the filter dot on the status trigger when a status is active", () => {
    setSearchParams("status=working");
    render(<TeamPage />);
    expect(screen.getByTestId("team-status-filter-dot")).toBeInTheDocument();
  });

  it("does not render the filter dot when status is All", () => {
    render(<TeamPage />);
    expect(screen.queryByTestId("team-status-filter-dot")).not.toBeInTheDocument();
  });

  it("renders chip rail with status + search chips when both filters are active", () => {
    setSearchParams("status=working&q=alpha");
    render(<TeamPage />);
    const rail = screen.getByTestId("team-filter-chip-rail");
    expect(within(rail).getByTestId("team-filter-chip-status")).toHaveTextContent(
      /Status: Working/,
    );
    expect(within(rail).getByTestId("team-filter-chip-search")).toHaveTextContent(
      /Search: "alpha"/,
    );
    expect(within(rail).getByTestId("team-filter-chip-rail-clear-all")).toBeInTheDocument();
  });

  it("hides the chip rail when no filters are active", () => {
    render(<TeamPage />);
    expect(screen.queryByTestId("team-filter-chip-rail")).not.toBeInTheDocument();
  });

  it("clears the status filter from the chip × button", async () => {
    setSearchParams("status=working");
    const user = userEvent.setup();
    render(<TeamPage />);
    await user.click(screen.getByTestId("team-filter-chip-status-remove"));
    expect(replaceMock).toHaveBeenCalled();
    const lastCall = replaceMock.mock.calls.at(-1);
    expect(lastCall?.[0]).toBe("/team");
  });

  it("clears every active dimension when 'Clear all' is clicked", async () => {
    setSearchParams("status=working&q=alpha");
    const user = userEvent.setup();
    render(<TeamPage />);
    await user.click(screen.getByTestId("team-filter-chip-rail-clear-all"));
    expect(replaceMock).toHaveBeenCalled();
    const lastCall = replaceMock.mock.calls.at(-1);
    expect(lastCall?.[0]).toBe("/team");
  });

  it("subtitle reflects the filtered count when a filter is active", () => {
    setSearchParams("status=working");
    render(<TeamPage />);
    // Only wkr_alpha has status=working in the sample.
    expect(screen.getByTestId("team-subtitle")).toHaveTextContent(
      /1 teammate working/,
    );
  });
});
