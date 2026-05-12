import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { WorkerHero, type WorkerHeroAction } from "./WorkerHero";
import {
  DEFAULT_DELEGATOR_PERMISSIONS,
  type Worker,
} from "@/lib/types/worker";

function makeWorker(overrides: Partial<Worker> = {}): Worker {
  return {
    id: "priya",
    name: "Priya",
    role: { tagline: "Senior Recruiter (AI)", vertical: "recruiter" },
    status: "working",
    department: { id: "people-and-culture", name: "People & Culture" },
    team: { id: "people-ops", name: "People Ops" },
    kpiToday: { label: "interviews booked", value: "3" },
    permissions: DEFAULT_DELEGATOR_PERMISSIONS,
    ...overrides,
  };
}

describe("WorkerHero", () => {
  it("renders the load-bearing identity fields", () => {
    render(<WorkerHero worker={makeWorker()} />);
    expect(screen.getByTestId("worker-name")).toHaveTextContent("Priya");
    expect(screen.getByTestId("worker-role")).toHaveTextContent("Senior Recruiter (AI)");
    expect(screen.getByTestId("worker-org-line")).toHaveTextContent("People & Culture · People Ops");
    expect(screen.getByTestId("worker-kpi")).toHaveTextContent("3 interviews booked");
    expect(screen.getByTestId("worker-status-pill")).toHaveTextContent("Working");
  });

  it("omits the org line when department + team are absent", () => {
    render(
      <WorkerHero
        worker={makeWorker({ department: undefined, team: undefined })}
      />,
    );
    expect(screen.queryByTestId("worker-org-line")).not.toBeInTheDocument();
  });

  it("omits the KPI line when kpiToday is missing", () => {
    render(<WorkerHero worker={makeWorker({ kpiToday: undefined })} />);
    expect(screen.queryByTestId("worker-kpi")).not.toBeInTheDocument();
  });

  it("renders distinct status pill labels per worker status", () => {
    const { rerender } = render(<WorkerHero worker={makeWorker({ status: "blocked" })} />);
    expect(screen.getByTestId("worker-status-pill")).toHaveTextContent("Blocked");
    rerender(<WorkerHero worker={makeWorker({ status: "paused" })} />);
    expect(screen.getByTestId("worker-status-pill")).toHaveTextContent("Paused");
    rerender(<WorkerHero worker={makeWorker({ status: "idle" })} />);
    expect(screen.getByTestId("worker-status-pill")).toHaveTextContent("Idle");
  });

  it("opens the overflow menu with all five action items", async () => {
    const user = userEvent.setup();
    render(<WorkerHero worker={makeWorker()} />);
    expect(screen.queryByTestId("worker-overflow-menu")).not.toBeInTheDocument();
    await user.click(screen.getByTestId("worker-overflow"));
    const menu = await screen.findByTestId("worker-overflow-menu");
    expect(within(menu).getByTestId("worker-action-pause")).toBeInTheDocument();
    expect(within(menu).getByTestId("worker-action-disable-duties")).toBeInTheDocument();
    expect(within(menu).getByTestId("worker-action-archive")).toBeInTheDocument();
    expect(within(menu).getByTestId("worker-action-export")).toBeInTheDocument();
    expect(within(menu).getByTestId("worker-action-view-as-operator")).toBeInTheDocument();
  });

  it("fires onAction with the selected action id", async () => {
    const user = userEvent.setup();
    const onAction = vi.fn<(action: WorkerHeroAction) => void>();
    render(<WorkerHero worker={makeWorker()} onAction={onAction} />);
    await user.click(screen.getByTestId("worker-overflow"));
    await user.click(await screen.findByTestId("worker-action-archive"));
    expect(onAction).toHaveBeenCalledWith("archive");
  });

  it("relabels Pause → Resume when the worker is paused", async () => {
    const user = userEvent.setup();
    render(<WorkerHero worker={makeWorker({ status: "paused" })} />);
    await user.click(screen.getByTestId("worker-overflow"));
    expect(await screen.findByTestId("worker-action-pause")).toHaveTextContent("Resume");
  });

  it("disables permission-gated items when permissions deny them", async () => {
    const user = userEvent.setup();
    render(
      <WorkerHero
        worker={makeWorker({
          permissions: {
            ...DEFAULT_DELEGATOR_PERMISSIONS,
            canPauseResume: false,
            canArchive: false,
            canEditDuties: false,
          },
        })}
      />,
    );
    await user.click(screen.getByTestId("worker-overflow"));
    expect(await screen.findByTestId("worker-action-pause")).toHaveAttribute(
      "data-disabled",
    );
    expect(screen.getByTestId("worker-action-archive")).toHaveAttribute("data-disabled");
    expect(screen.getByTestId("worker-action-disable-duties")).toHaveAttribute(
      "data-disabled",
    );
  });
});
