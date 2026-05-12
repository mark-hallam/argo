import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import type { Worker, WorkerStatus } from "@/lib/types/worker";
import type { WorkerActionResult } from "@/lib/api/workers";

const pushMock = vi.hoisted(() => vi.fn());

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: pushMock,
    replace: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  usePathname: () => "/team",
  useSearchParams: () => new URLSearchParams(),
}));

const pauseSpy = vi.hoisted(() => vi.fn<(id: string) => Promise<WorkerActionResult>>());
const resumeSpy = vi.hoisted(() => vi.fn<(id: string) => Promise<WorkerActionResult>>());

vi.mock("@/lib/api/workers", () => ({
  pauseWorker: (id: string) => pauseSpy(id),
  resumeWorker: (id: string) => resumeSpy(id),
}));

import { TeammateCard } from "./TeammateCard";

function makeWorker(overrides: Partial<Worker> = {}): Worker {
  return {
    id: "wkr_test",
    name: "Mariah",
    role: { tagline: "lists and re-prices items", vertical: "marketplace" },
    status: "working" as WorkerStatus,
    headline: {
      bullets: ["8 replies sent (2 waiting your OK)", "$340 in pending offers"],
      tomorrow: "morning routine at 06:30",
    },
    lastActiveAt: "2026-05-12T10:00:00.000Z",
    ...overrides,
  };
}

function wrap(node: ReactNode, client?: QueryClient) {
  const qc =
    client ??
    new QueryClient({
      defaultOptions: {
        queries: { retry: false, refetchOnWindowFocus: false },
        mutations: { retry: false },
      },
    });
  return {
    qc,
    ...render(<QueryClientProvider client={qc}>{node}</QueryClientProvider>),
  };
}

async function openMenuByButton(user: ReturnType<typeof userEvent.setup>, worker: Worker) {
  await user.click(
    screen.getByTestId(`teammate-card-menu-trigger-${worker.id}`),
  );
  return await screen.findByTestId(`teammate-card-menu-${worker.id}`);
}

beforeEach(() => {
  pushMock.mockClear();
  pauseSpy.mockReset();
  resumeSpy.mockReset();
  pauseSpy.mockResolvedValue({ ok: true, workerId: "wkr_test", status: "paused" });
  resumeSpy.mockResolvedValue({ ok: true, workerId: "wkr_test", status: "idle" });
});

describe("TeammateCard", () => {
  it.each<[WorkerStatus, string]>([
    ["working", "Working"],
    ["waiting", "Waiting on you"],
    ["idle", "Idle"],
    ["blocked", "Blocked"],
    ["paused", "Paused"],
  ])("renders status pill for %s", (status, label) => {
    wrap(<TeammateCard worker={makeWorker({ status })} />);
    expect(screen.getByTestId("teammate-status-wkr_test")).toHaveTextContent(label);
  });

  it("renders avatar, name, today bullets, and tomorrow line", () => {
    wrap(<TeammateCard worker={makeWorker()} />);
    expect(screen.getByText("Mariah")).toBeInTheDocument();
    expect(screen.getByText("lists and re-prices items")).toBeInTheDocument();
    const today = screen.getByTestId("teammate-today-wkr_test");
    expect(within(today).getByText(/8 replies sent/)).toBeInTheDocument();
    expect(within(today).getByText(/\$340 in pending offers/)).toBeInTheDocument();
    expect(screen.getByTestId("teammate-tomorrow-wkr_test")).toHaveTextContent(
      /morning routine at 06:30/,
    );
  });

  it("opens the context menu when the ⋮ button is clicked", async () => {
    const user = userEvent.setup();
    wrap(<TeammateCard worker={makeWorker()} />);
    const content = await openMenuByButton(user, makeWorker());
    expect(within(content).getByText(/Pause Mariah/)).toBeInTheDocument();
  });

  it("opens the context menu on right-click of the card body", async () => {
    const user = userEvent.setup();
    wrap(<TeammateCard worker={makeWorker()} />);
    await user.pointer({
      keys: "[MouseRight]",
      target: screen.getByTestId("teammate-wkr_test"),
    });
    expect(
      await screen.findByTestId("teammate-card-menu-wkr_test"),
    ).toBeInTheDocument();
  });

  it("shows the Resume entry when status is paused", async () => {
    const user = userEvent.setup();
    wrap(<TeammateCard worker={makeWorker({ status: "paused" })} />);
    const content = await openMenuByButton(user, makeWorker());
    expect(within(content).getByText(/Resume Mariah/)).toBeInTheDocument();
    expect(within(content).queryByText(/Pause Mariah/)).not.toBeInTheDocument();
  });

  it("pause action: confirm dialog → POST → optimistic cache update", async () => {
    const user = userEvent.setup();
    const qc = new QueryClient({
      defaultOptions: {
        queries: { retry: false, refetchOnWindowFocus: false },
        mutations: { retry: false },
      },
    });
    const worker = makeWorker();
    qc.setQueryData(["team"], { workers: [worker], source: "mock-offline" });

    wrap(<TeammateCard worker={worker} />, qc);

    await openMenuByButton(user, worker);
    await user.click(screen.getByTestId("teammate-card-menu-pause-wkr_test"));

    // Confirm dialog
    const dialog = await screen.findByTestId("alert-dialog");
    expect(within(dialog).getByText(/Pause Mariah\?/)).toBeInTheDocument();

    await user.click(screen.getByTestId("alert-dialog-confirm"));

    // Optimistic flip + network call
    await waitFor(() => expect(pauseSpy).toHaveBeenCalledWith("wkr_test"));
    const cached = qc.getQueryData<{ workers: Worker[] }>(["team"]);
    expect(cached?.workers[0]?.status).toBe("paused");
  });

  it("resume action: confirm dialog → POST → optimistic cache update", async () => {
    const user = userEvent.setup();
    const qc = new QueryClient({
      defaultOptions: {
        queries: { retry: false, refetchOnWindowFocus: false },
        mutations: { retry: false },
      },
    });
    const worker = makeWorker({ status: "paused" });
    qc.setQueryData(["team"], { workers: [worker], source: "mock-offline" });

    wrap(<TeammateCard worker={worker} />, qc);

    await openMenuByButton(user, worker);
    await user.click(screen.getByTestId("teammate-card-menu-resume-wkr_test"));

    const dialog = await screen.findByTestId("alert-dialog");
    expect(within(dialog).getByText(/Resume Mariah\?/)).toBeInTheDocument();
    await user.click(screen.getByTestId("alert-dialog-confirm"));

    await waitFor(() => expect(resumeSpy).toHaveBeenCalledWith("wkr_test"));
    const cached = qc.getQueryData<{ workers: Worker[] }>(["team"]);
    expect(cached?.workers[0]?.status).toBe("idle");
  });

  it("rolls back the optimistic update if the pause mutation rejects", async () => {
    pauseSpy.mockRejectedValueOnce(new Error("boom"));
    const user = userEvent.setup();
    const qc = new QueryClient({
      defaultOptions: {
        queries: { retry: false, refetchOnWindowFocus: false },
        mutations: { retry: false },
      },
    });
    const worker = makeWorker();
    qc.setQueryData(["team"], { workers: [worker], source: "mock-offline" });

    wrap(<TeammateCard worker={worker} />, qc);
    await openMenuByButton(user, worker);
    await user.click(screen.getByTestId("teammate-card-menu-pause-wkr_test"));
    await user.click(screen.getByTestId("alert-dialog-confirm"));

    await waitFor(() => {
      const cached = qc.getQueryData<{ workers: Worker[] }>(["team"]);
      expect(cached?.workers[0]?.status).toBe("working");
    });
  });

  it("clicking the card body routes to /workers/<id>", async () => {
    const user = userEvent.setup();
    wrap(<TeammateCard worker={makeWorker()} />);
    await user.click(screen.getByTestId("teammate-wkr_test"));
    expect(pushMock).toHaveBeenCalledWith("/workers/wkr_test");
  });

  it("clicking the Open profile button routes to /workers/<id> once", async () => {
    const user = userEvent.setup();
    wrap(<TeammateCard worker={makeWorker()} />);
    await user.click(screen.getByTestId("teammate-open-profile-wkr_test"));
    expect(pushMock).toHaveBeenCalledWith("/workers/wkr_test");
    expect(pushMock).toHaveBeenCalledTimes(1);
  });

  it("focused card routes on Enter", async () => {
    wrap(<TeammateCard worker={makeWorker()} />);
    const card = screen.getByTestId("teammate-wkr_test");
    act(() => card.focus());
    await userEvent.keyboard("{Enter}");
    expect(pushMock).toHaveBeenCalledWith("/workers/wkr_test");
  });
});
