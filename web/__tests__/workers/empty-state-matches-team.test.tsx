import { render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { EXAMPLE_ROLES } from "@/lib/hire/examples";
import type { Worker } from "@/lib/types/worker";

/**
 * Regression for `web-workers-empty-state-converge-with-team`.
 *
 * Before the brief, `/workers` rendered a sparse single-CTA
 * `<EmptyState>` while `/team` rendered the rich 5-prompt
 * `<TeamEmptyState>` grid. The buyer saw two different first-touch
 * empty states for the same concept. This test pins the convergence:
 * with an empty roster, both routes must surface the same five
 * starter prompts with the same `?example=<id>` hrefs.
 */

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
  usePathname: () => "/",
  useSearchParams: () => searchParams,
}));

type FetchTeamResultShape = {
  workers: Worker[];
  source: "gateway" | "mock-offline" | "mock-bypass";
};
type QueryShape = {
  data: FetchTeamResultShape | undefined;
  isPending: boolean;
};

const useTeamMock = vi.fn<() => QueryShape>();

vi.mock("@/lib/hooks/use-team", () => ({
  useTeam: () => useTeamMock(),
}));

import WorkersPage from "@/app/(app)/workers/page";
import TeamPage from "@/app/(app)/team/page";

function hrefsFromTiles(): string[] {
  const grid = screen.getByTestId("team-empty-state-tiles");
  return within(grid)
    .getAllByRole("link")
    .map((tile) => tile.getAttribute("href") ?? "");
}

beforeEach(() => {
  searchParams = new URLSearchParams();
  replaceMock.mockClear();
  useTeamMock.mockReset();
  useTeamMock.mockReturnValue({
    data: { workers: [], source: "mock-offline" },
    isPending: false,
  });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("workers/team empty-state convergence", () => {
  it("/workers and /team render the same 5 starter-prompt hrefs when empty", () => {
    const { unmount } = render(<WorkersPage />);
    const workersHrefs = hrefsFromTiles();
    expect(workersHrefs).toHaveLength(EXAMPLE_ROLES.length);
    unmount();

    render(<TeamPage />);
    const teamHrefs = hrefsFromTiles();
    expect(teamHrefs).toHaveLength(EXAMPLE_ROLES.length);

    expect(workersHrefs).toEqual(teamHrefs);
    for (const role of EXAMPLE_ROLES) {
      expect(workersHrefs).toContain(`/hire?example=${role.id}`);
    }
  });

  it("/workers exposes every locked starter-prompt title", () => {
    render(<WorkersPage />);
    const grid = screen.getByTestId("team-empty-state-tiles");
    for (const role of EXAMPLE_ROLES) {
      expect(within(grid).getByText(role.title)).toBeInTheDocument();
    }
  });

  it("/workers no longer shows the old sparse single-CTA copy", () => {
    render(<WorkersPage />);
    expect(screen.queryByText(/no teammates yet/i)).not.toBeInTheDocument();
    expect(
      screen.queryByText(/hire your first ai worker to start shipping/i),
    ).not.toBeInTheDocument();
  });

  it("/workers empty-state uses the locked '/team' heading copy", () => {
    render(<WorkersPage />);
    expect(
      screen.getByRole("heading", {
        level: 2,
        name: /hire your first ai teammate\./i,
      }),
    ).toBeInTheDocument();
  });
});
