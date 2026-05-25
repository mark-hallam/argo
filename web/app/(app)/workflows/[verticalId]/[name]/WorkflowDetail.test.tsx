import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import WorkflowDetailPage from "./page";

const mocks = vi.hoisted(() => ({
  executeWorkflow: vi.fn(),
  fetchWorkflowRuns: vi.fn(),
  replace: vi.fn(),
  refresh: vi.fn(),
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
  useWorkflow: vi.fn(),
}));

vi.mock("next/navigation", () => ({
  usePathname: () => "/workflows/gtm-engineer/post_listing",
  useRouter: () => ({
    replace: mocks.replace,
    refresh: mocks.refresh,
  }),
  useSearchParams: () => new URLSearchParams(),
}));

vi.mock("next/link", () => ({
  default: ({
    href,
    children,
    ...rest
  }: { href: string; children: ReactNode } & Record<string, unknown>) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

vi.mock("@/lib/hooks/use-workflows", () => ({
  useWorkflow: mocks.useWorkflow,
}));

vi.mock("@/lib/api/workflow-execute", () => ({
  executeWorkflow: mocks.executeWorkflow,
}));

vi.mock("@/lib/api/workflow-runs", () => ({
  WORKFLOW_RUN_STATUSES: ["success", "failed", "running", "queued"],
  fetchWorkflowRuns: mocks.fetchWorkflowRuns,
}));

vi.mock("@/components/skycot/WorkflowProposeModal", () => ({
  WorkflowProposeModal: () => null,
}));

vi.mock("@/components/skycot/workflows/WorkflowVersionsPanel", () => ({
  WorkflowVersionsPanel: () => null,
}));

vi.mock("./add-as-schedule-drawer", () => ({
  AddAsScheduleDrawer: () => null,
}));

vi.mock("sonner", () => ({
  toast: {
    error: mocks.toastError,
    success: mocks.toastSuccess,
  },
}));

const workflow = {
  name: "post_listing",
  vertical: "gtm-engineer",
  version: "v3",
  description: "Validate inventory, generate copy, and post the listing.",
  risk: "approval-required",
  steps: [],
};

const runsSummary = {
  vertical_id: "gtm-engineer",
  workflow_name: "post_listing",
  window_days: 7,
  run_count: 0,
  success_rate: null,
  avg_duration_ms: 0,
  p95_duration_ms: 0,
  last_run_at: null,
  page: 1,
  page_size: 25,
  total_filtered: 0,
  runs: [],
};

interface RouteParams {
  verticalId: string;
  name: string;
}

function fulfilledParams(value: RouteParams): Promise<RouteParams> {
  const params = Promise.resolve(value) as Promise<RouteParams> & {
    status: "fulfilled";
    value: RouteParams;
  };
  params.status = "fulfilled";
  params.value = value;
  return params;
}

function renderPage() {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={client}>
      <WorkflowDetailPage
        params={fulfilledParams({
          verticalId: "gtm-engineer",
          name: "post_listing",
        })}
      />
    </QueryClientProvider>,
  );
}

describe("WorkflowDetailPage", () => {
  const staleRunLabel = "Run " + "workflow";

  beforeEach(() => {
    mocks.executeWorkflow.mockReset();
    mocks.fetchWorkflowRuns.mockReset();
    mocks.replace.mockReset();
    mocks.refresh.mockReset();
    mocks.toastError.mockReset();
    mocks.toastSuccess.mockReset();
    mocks.useWorkflow.mockReset();

    mocks.executeWorkflow.mockResolvedValue({
      run_id: "run_1234567890",
      vertical_id: "gtm-engineer",
      workflow_name: "post_listing",
      ok: true,
      task_verdicts: [],
      dispatched_at: "2026-05-25T13:00:00.000Z",
    });
    mocks.fetchWorkflowRuns.mockResolvedValue({
      summary: runsSummary,
      source: "gateway",
    });
    mocks.useWorkflow.mockReturnValue({
      data: { workflow, source: "gateway" },
      isError: false,
      isLoading: false,
    });
  });

  it("pins the workflow run action label to Run now", async () => {
    renderPage();

    const runButton = await screen.findByTestId("wfd-run-workflow");

    expect(runButton).toBeEnabled();
    expect(runButton.textContent?.trim()).toMatchInlineSnapshot(`"Run now"`);
    expect(screen.queryByText(staleRunLabel)).not.toBeInTheDocument();
  });

  it("keeps the in-flight label as Running…", async () => {
    const user = userEvent.setup();
    let resolveRun: (value: unknown) => void = () => {};
    mocks.executeWorkflow.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveRun = resolve;
      }),
    );
    renderPage();

    const runButton = await screen.findByTestId("wfd-run-workflow");
    await user.click(runButton);

    expect(runButton.textContent?.trim()).toMatchInlineSnapshot(`"Running…"`);

    resolveRun({
      run_id: "run_1234567890",
      vertical_id: "gtm-engineer",
      workflow_name: "post_listing",
      ok: true,
      task_verdicts: [],
      dispatched_at: "2026-05-25T13:00:00.000Z",
    });
    await waitFor(() => expect(runButton).toHaveTextContent("Run now"));
  });
});
