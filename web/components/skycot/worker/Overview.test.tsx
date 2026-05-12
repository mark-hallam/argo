import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WorkerOverviewContent } from "@/components/skycot/worker/WorkerOverviewContent";
import { WorkerOverviewNotFoundError } from "@/lib/api/worker-overview";
import { WhatSheDoesCard } from "@/components/skycot/worker/WhatSheDoesCard";
import { DutiesCard } from "@/components/skycot/worker/DutiesCard";
import { KpiCard } from "@/components/skycot/worker/KpiCard";
import { RecentActivityCard } from "@/components/skycot/worker/RecentActivityCard";
import { KpiSparkline } from "@/components/ui/charts";
import {
  fetchWorkerOverview,
  type FetchWorkerOverviewResult,
  type WorkerOverview,
} from "@/lib/api/worker-overview";
import { fetchWorker } from "@/lib/api/workers";
import {
  DEFAULT_DELEGATOR_PERMISSIONS,
  type Worker,
} from "@/lib/types/worker";

vi.mock("@/lib/api/worker-overview", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/worker-overview")>(
    "@/lib/api/worker-overview",
  );
  return { ...actual, fetchWorkerOverview: vi.fn() };
});
vi.mock("@/lib/api/workers", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/workers")>(
    "@/lib/api/workers",
  );
  return { ...actual, fetchWorker: vi.fn() };
});

const mockedFetchOverview = vi.mocked(fetchWorkerOverview);
const mockedFetchWorker = vi.mocked(fetchWorker);

function sampleOverview(): WorkerOverview {
  return {
    charter: "Priya helps the People Ops team. She screens candidates. She asks the hiring manager.",
    duties: [
      { id: "d1", label: "Screen inbound applicants", schedule: "as applications arrive", enabled: true },
      { id: "d2", label: "Draft rejection notes", schedule: "weekdays 08:30", enabled: false },
    ],
    kpis: [
      { id: "k1", label: "candidates screened today", value: "8", series: [4, 6, 5, 7, 9, 11, 8], deltaPct: 12, trend: "up", tone: "success" },
      { id: "k2", label: "interviews booked this week", value: "14", series: [2, 3, 2, 3, 2, 1, 1], deltaPct: -8, trend: "down", tone: "warning" },
      { id: "k3", label: "pending rejection drafts", value: "3", series: [5, 4, 4, 3, 3, 3, 3], deltaPct: -22, trend: "down", tone: "warning" },
      { id: "k4", label: "time-to-first-response avg", value: "4m", series: [9, 7, 6, 6, 5, 5, 4], deltaPct: -38, trend: "down", tone: "warning" },
    ],
    recentActivity: [
      { id: "ap_1", at: new Date().toISOString(), verb: "booked interview", status: "approved", href: "/audit/action/ap_1" },
      { id: "ap_2", at: new Date().toISOString(), verb: "screened candidate", status: "auto-sent", href: "/audit/action/ap_2" },
      { id: "ap_3", at: new Date().toISOString(), verb: "drafted rejection", status: "needs review", href: "/audit/action/ap_3" },
      { id: "ap_4", at: new Date().toISOString(), verb: "asked hiring manager", status: "awaiting", href: "/audit/action/ap_4" },
      { id: "ap_5", at: new Date().toISOString(), verb: "scheduled follow-up", status: "auto-sent", href: "/audit/action/ap_5" },
    ],
  };
}

function sampleWorker(): Worker {
  return {
    id: "wkr_priya",
    name: "Priya",
    role: { tagline: "Senior Recruiter (AI)", vertical: "recruiter" },
    status: "working",
    permissions: DEFAULT_DELEGATOR_PERMISSIONS,
  };
}

function makeQueryClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } });
}

function PageWrapper({ workerId = "wkr_priya" }: { workerId?: string }) {
  const client = makeQueryClient();
  return (
    <QueryClientProvider client={client}>
      <WorkerOverviewContent workerId={workerId} />
    </QueryClientProvider>
  );
}

beforeEach(() => {
  mockedFetchWorker.mockResolvedValue(sampleWorker());
  mockedFetchOverview.mockResolvedValue({
    overview: sampleOverview(),
    source: "mock-offline",
  } satisfies FetchWorkerOverviewResult);
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("WorkerOverviewPage — all four cards render", () => {
  it("renders WhatSheDoes, Duties, KPIs, and RecentActivity in order", async () => {
    render(<PageWrapper />);
    expect(await screen.findByTestId("card-what-she-does")).toBeInTheDocument();
    expect(screen.getByTestId("card-duties")).toBeInTheDocument();
    expect(screen.getByTestId("card-kpis")).toBeInTheDocument();
    expect(screen.getByTestId("card-recent-activity")).toBeInTheDocument();
  });

  it("shows the loading skeleton while the overview query is pending", async () => {
    mockedFetchOverview.mockImplementation(() => new Promise(() => {}));
    render(<PageWrapper />);
    expect(await screen.findByTestId("worker-overview-loading")).toBeInTheDocument();
    expect(screen.getByTestId("card-what-she-does-skeleton")).toBeInTheDocument();
    expect(screen.getByTestId("card-duties-skeleton")).toBeInTheDocument();
    expect(screen.getByTestId("card-kpis-skeleton")).toBeInTheDocument();
    expect(screen.getByTestId("card-recent-activity-skeleton")).toBeInTheDocument();
  });

  it("renders an error state with retry when the overview fetch fails", async () => {
    mockedFetchOverview.mockRejectedValue(new WorkerOverviewNotFoundError("wkr_priya"));
    render(<PageWrapper />);
    const errorCard = await screen.findByTestId("worker-overview-error");
    expect(errorCard).toHaveTextContent(/not found/i);
    expect(within(errorCard).getByRole("button", { name: /try again/i })).toBeInTheDocument();
  });
});

describe("WhatSheDoesCard — inline edit", () => {
  it("commits the charter on Enter when canEdit is true", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    render(
      <WhatSheDoesCard
        charter="Original charter."
        canEdit
        onSave={onSave}
      />,
    );
    await user.click(screen.getByTestId("charter-text"));
    const editor = await screen.findByTestId("charter-editor");
    await user.clear(editor);
    await user.type(editor, "New charter copy.");
    await user.keyboard("{Enter}");
    expect(onSave).toHaveBeenCalledWith("New charter copy.");
  });

  it("renders the Adjust button and stubs the action through onAdjust", async () => {
    const user = userEvent.setup();
    const onAdjust = vi.fn();
    render(
      <WhatSheDoesCard
        charter="Charter."
        canEdit
        onAdjust={onAdjust}
      />,
    );
    await user.click(screen.getByTestId("charter-adjust"));
    expect(onAdjust).toHaveBeenCalledOnce();
  });

  it("does not enter edit mode when canEdit is false", async () => {
    const user = userEvent.setup();
    render(<WhatSheDoesCard charter="Charter." canEdit={false} />);
    await user.click(screen.getByTestId("charter-text"));
    expect(screen.queryByTestId("charter-editor")).not.toBeInTheDocument();
  });
});

describe("DutiesCard — toggle + add", () => {
  it("calls onToggle with the new value when the toggle is clicked", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(
      <DutiesCard
        canEdit
        onToggle={onToggle}
        duties={[
          { id: "d1", label: "Screen applicants", schedule: "as needed", enabled: true },
        ]}
      />,
    );
    await user.click(screen.getByTestId("duty-toggle-d1"));
    expect(onToggle).toHaveBeenCalledWith("d1", false);
  });

  it("fires onAddDuty when '+ Add responsibility' is clicked", async () => {
    const user = userEvent.setup();
    const onAddDuty = vi.fn();
    render(<DutiesCard canEdit duties={[]} onAddDuty={onAddDuty} />);
    await user.click(screen.getByTestId("duties-add"));
    expect(onAddDuty).toHaveBeenCalledOnce();
  });
});

describe("KpiCard + KpiSparkline", () => {
  it("renders the trend chip and sparkline for each KPI", () => {
    render(<KpiCard kpis={sampleOverview().kpis} />);
    expect(screen.getByTestId("kpi-row-k1")).toHaveAttribute("data-tone", "success");
    expect(screen.getByTestId("kpi-row-k2")).toHaveAttribute("data-tone", "warning");
    expect(screen.getAllByTestId("kpi-sparkline").length).toBe(4);
    expect(screen.getByTestId("kpi-value-k1")).toHaveTextContent("8");
  });

  it("renders a non-empty path even when the series is degenerate", () => {
    const { container } = render(<KpiSparkline data={[5, 5, 5, 5]} tone="muted" />);
    const path = container.querySelector("path");
    expect(path).toBeTruthy();
    expect(path?.getAttribute("d") ?? "").toMatch(/^M/);
  });
});

describe("RecentActivityCard", () => {
  it("renders 5 rows, each linking to the audit drill-in", () => {
    const rows = sampleOverview().recentActivity;
    render(<RecentActivityCard workerId="wkr_priya" rows={rows} />);
    expect(screen.getAllByTestId(/activity-row-/).length).toBe(5);
    rows.forEach((row) => {
      const link = screen
        .getByTestId(`activity-row-${row.id}`)
        .querySelector("a");
      expect(link?.getAttribute("href")).toBe(row.href);
    });
  });

  it("routes 'See all in Activity →' to the worker's Activity tab", () => {
    render(<RecentActivityCard workerId="wkr_priya" rows={sampleOverview().recentActivity} />);
    const seeAll = screen.getByTestId("recent-activity-see-all");
    expect(seeAll.getAttribute("href")).toBe("/workers/wkr_priya/activity");
  });

  it("renders an empty-state message when there is no activity", () => {
    render(<RecentActivityCard workerId="wkr_priya" rows={[]} />);
    expect(screen.getByTestId("card-recent-activity")).toHaveTextContent(
      /No activity yet/i,
    );
  });
});

describe("Overview page wiring", () => {
  it("opens the RetrainDialog (E4-g) when Adjust is clicked", async () => {
    const user = userEvent.setup();
    render(<PageWrapper />);
    await user.click(await screen.findByTestId("charter-adjust"));
    expect(await screen.findByTestId("retrain-dialog")).toBeInTheDocument();
    expect(screen.getByTestId("retrain-step-capture")).toBeInTheDocument();
  });

  it("opens a stub toast pointing at E4-h when Add responsibility is clicked", async () => {
    const user = userEvent.setup();
    render(<PageWrapper />);
    await user.click(await screen.findByTestId("duties-add"));
    await waitFor(() =>
      expect(screen.getByTestId("overview-toast")).toHaveTextContent(/E4-h/),
    );
  });
});
