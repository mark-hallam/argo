import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WorkerOverviewContent } from "@/components/skycot/worker/WorkerOverviewContent";
import { WorkerOverviewNotFoundError } from "@/lib/api/worker-overview";
import { WhatTheyDoCard } from "@/components/skycot/worker/WhatTheyDoCard";
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
    charter: "Priya helps the People Ops team. They screen candidates. They ask the hiring manager.",
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
  it("renders WhatTheyDo, Duties, KPIs, and RecentActivity in order", async () => {
    render(<PageWrapper />);
    expect(await screen.findByTestId("card-what-they-do")).toBeInTheDocument();
    expect(screen.getByTestId("card-duties")).toBeInTheDocument();
    expect(screen.getByTestId("card-kpis")).toBeInTheDocument();
    expect(screen.getByTestId("card-recent-activity")).toBeInTheDocument();
  });

  it("shows the loading skeleton while the overview query is pending", async () => {
    mockedFetchOverview.mockImplementation(() => new Promise(() => {}));
    render(<PageWrapper />);
    expect(await screen.findByTestId("worker-overview-loading")).toBeInTheDocument();
    expect(screen.getByTestId("card-what-they-do-skeleton")).toBeInTheDocument();
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

describe("WhatTheyDoCard — inline edit", () => {
  it("commits the charter on Enter when canEdit is true", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    render(
      <WhatTheyDoCard
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

  it("renders the Adjust button (dialog opens via WorkerOverviewContent pin)", () => {
    render(<WhatTheyDoCard charter="Charter." canEdit />);
    // The Adjust button mounts the RetrainDialog internally when
    // workerId is set; that integration is pinned by
    // "opens the RetrainDialog (E4-g) when Adjust is clicked" below.
    // Here we just guard that the button is reachable.
    expect(screen.getByTestId("charter-adjust")).toBeInTheDocument();
  });

  it("does not enter edit mode when canEdit is false", async () => {
    const user = userEvent.setup();
    render(<WhatTheyDoCard charter="Charter." canEdit={false} />);
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

  it("fires onAddDuty when '+ Add schedule' is clicked", async () => {
    const user = userEvent.setup();
    const onAddDuty = vi.fn();
    render(<DutiesCard canEdit duties={[]} onAddDuty={onAddDuty} />);
    await user.click(screen.getByTestId("duties-add"));
    expect(onAddDuty).toHaveBeenCalledOnce();
  });

  // Operator vocab lock (memory project_vocabulary_duty_vs_schedule,
  // 2026-05-23) — visible card copy uses "Schedule", never
  // "Responsibility". The aria-label / data-testid surfaces (which
  // keep "duty" for code-side stability) are not part of textContent,
  // so this guard stays scoped to operator-visible strings.
  it("renders no 'Responsibility' / 'responsibilities' copy in visible text", () => {
    render(
      <DutiesCard
        canEdit
        duties={[
          { id: "d1", label: "Screen applicants", schedule: "as needed", enabled: true },
        ]}
      />,
    );
    const card = screen.getByTestId("card-duties");
    expect(card.textContent ?? "").not.toMatch(/responsibilit(y|ies)/i);
    expect(card.textContent ?? "").toMatch(/Schedules/);
    expect(card.textContent ?? "").toMatch(/Add schedule/);
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

  // Brief: fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak.
  // Clicking Add schedule opens the real AddScheduleDialog
  // (DutiesCard mounts it directly). Previously this fired a stub toast
  // whose body leaked the engineering brief-ID "E4-h" into the camera-
  // visible surface.
  it("opens the AddScheduleDialog when Add schedule is clicked", async () => {
    const user = userEvent.setup();
    render(<PageWrapper />);
    await user.click(await screen.findByTestId("duties-add"));
    expect(await screen.findByTestId("duty-dialog")).toBeInTheDocument();
  });

  it("renders the duty toggle as disabled (no fake-save theater)", async () => {
    render(<PageWrapper />);
    const toggle = await screen.findByTestId("duty-toggle-d1");
    expect(toggle).toBeDisabled();
  });

  it("renders the duty body as disabled (no toast on click)", async () => {
    render(<PageWrapper />);
    const body = await screen.findByTestId("duty-edit-d1");
    expect(body).toBeDisabled();
  });

  // Anti-leak guard: the Overview surface must never render an
  // engineering brief-ID like "E4-h" in customer-visible text.
  it("never renders an engineering brief-ID (E[0-9]+-[a-z]) in customer text", async () => {
    render(<PageWrapper />);
    await screen.findByTestId("card-duties");
    // Grab full visible text content of the rendered overview panel.
    const panel = screen.getByTestId("worker-tab-panel-overview");
    expect(panel.textContent ?? "").not.toMatch(/\bE\d+-[a-z]\b/);
  });
});
