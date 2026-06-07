import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
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
  useSearchParams: () => searchParams,
}));

type TeamQueryShape = {
  data?: { workers: Worker[] };
  isPending: boolean;
  isError?: boolean;
  refetch?: () => void;
};

const useTeamMock = vi.fn<() => TeamQueryShape>();

vi.mock("@/lib/hooks/use-team", () => ({
  useTeam: () => useTeamMock(),
}));

vi.mock("@/components/empty-state/HireStarterPrompts", () => ({
  HireStarterPrompts: () => <div data-testid="hire-starter-prompts">Starter prompts</div>,
}));

vi.mock("@/components/skycot/TeammateGrid", () => ({
  TeammateGrid: ({ children }: { children: ReactNode }) => (
    <div data-testid="teammate-grid">{children}</div>
  ),
}));

vi.mock("@/components/skycot/TeammateCard", () => ({
  TeammateCard: ({
    worker,
    "data-testid": testId,
    className,
  }: {
    worker: Worker;
    "data-testid"?: string;
    className?: string;
  }) => (
    <div data-testid={testId ?? `teammate-${worker.id}`} className={className}>
      {worker.name}
    </div>
  ),
}));

import WorkersPage from "./page";

const WORKER: Worker = {
  id: "wkr_maya",
  name: "Maya",
  role: { tagline: "answers support", vertical: "support" },
  status: "idle",
  lastActiveAt: "2026-05-31T10:00:00.000Z",
};

function setSearchParams(qs: string) {
  searchParams = new URLSearchParams(qs);
}

beforeEach(() => {
  replaceMock.mockClear();
  useTeamMock.mockReset();
  setSearchParams("");
});

describe("WorkersPage", () => {
  it("renders the C2 workers hero and first-hire prompts for an empty roster", () => {
    useTeamMock.mockReturnValue({
      data: { workers: [] },
      isPending: false,
    });

    render(<WorkersPage />);

    expect(
      screen.getByRole("heading", { level: 1, name: /^workers$/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/workers · c2 roster/i)).toBeInTheDocument();
    expect(screen.getByText(/pick a starting point to hire a teammate/i)).toBeInTheDocument();
    expect(screen.getByTestId("workers-empty")).toBeInTheDocument();
    expect(screen.getByTestId("hire-starter-prompts")).toBeInTheDocument();
    expect(replaceMock).not.toHaveBeenCalled();
  });

  it("keeps the just-hired worker on /workers with the highlighted card", () => {
    setSearchParams("just-hired=1&new=wkr_maya");
    useTeamMock.mockReturnValue({
      data: { workers: [WORKER] },
      isPending: false,
    });

    render(<WorkersPage />);

    expect(
      screen.getByRole("heading", { level: 1, name: /^workers$/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/review the new worker/i)).toBeInTheDocument();
    expect(screen.getByText(/^Teammates$/)).toBeInTheDocument();
    const grid = screen.getByTestId("workers-grid");
    const card = within(grid).getByTestId("teammate-wkr_maya");
    expect(card).toHaveTextContent("Maya");
    expect(card.className).toContain("ring-[var(--primary)]");
    expect(replaceMock).not.toHaveBeenCalled();
  });

  it("redirects populated non-first-hire visits to the canonical team page", async () => {
    useTeamMock.mockReturnValue({
      data: { workers: [WORKER] },
      isPending: false,
    });

    render(<WorkersPage />);

    expect(
      screen.getByRole("heading", { level: 1, name: /^workers$/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/opening the canonical team roster/i)).toBeInTheDocument();
    await waitFor(() => expect(replaceMock).toHaveBeenCalledWith("/team"));
  });

  // Regression: on a gateway 500 the roster query errors. The page must
  // render an error+retry state (mirroring `/team`'s `isError` branch)
  // and must NEVER show the "hire your first teammate" empty grid — that
  // would tell a customer who just hired they have no team. Also must
  // not redirect to /team on error.
  it("renders an error+retry state on a failed roster fetch, not the hire-empty", async () => {
    const refetch = vi.fn();
    useTeamMock.mockReturnValue({
      data: undefined,
      isPending: false,
      isError: true,
      refetch,
    });

    render(<WorkersPage />);

    expect(screen.getByTestId("workers-error-state")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /trouble loading your team/i }),
    ).toBeInTheDocument();
    // The hire-empty CTA grid is absent on error.
    expect(screen.queryByTestId("hire-starter-prompts")).not.toBeInTheDocument();
    expect(screen.queryByTestId("workers-empty")).not.toBeInTheDocument();
    // No redirect to /team on error.
    expect(replaceMock).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: /try again/i }));
    expect(refetch).toHaveBeenCalledTimes(1);
  });
});
