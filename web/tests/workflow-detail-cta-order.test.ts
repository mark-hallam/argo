import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { createElement, type ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import WorkflowDetailPage from "@/app/(app)/workflows/[verticalId]/[name]/page";

const mocks = vi.hoisted(() => ({
  executeWorkflow: vi.fn(),
  fetchWorkflowRuns: vi.fn(),
  replace: vi.fn(),
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
  useWorkflow: vi.fn(),
}));

vi.mock("next/navigation", () => ({
  usePathname: () => "/workflows/gtm-engineer/post_listing",
  useRouter: () => ({
    replace: mocks.replace,
  }),
  useSearchParams: () => new URLSearchParams(),
}));

vi.mock("next/link", () => ({
  default: ({
    href,
    children,
    ...rest
  }: { href: string; children: ReactNode } & Record<string, unknown>) =>
    createElement("a", { href, ...rest }, children),
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

vi.mock("@/app/(app)/workflows/[verticalId]/[name]/add-as-schedule-drawer", () => ({
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
    createElement(
      QueryClientProvider,
      { client },
      createElement(WorkflowDetailPage, {
        params: fulfilledParams({
          verticalId: "gtm-engineer",
          name: "post_listing",
        }),
      }),
    ),
  );
}

describe("workflow detail CTA order", () => {
  beforeEach(() => {
    mocks.executeWorkflow.mockReset();
    mocks.fetchWorkflowRuns.mockReset();
    mocks.replace.mockReset();
    mocks.toastError.mockReset();
    mocks.toastSuccess.mockReset();
    mocks.useWorkflow.mockReset();

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

  it("makes Run now the first and only primary workflow detail CTA", async () => {
    renderPage();

    const actionBar = await screen.findByTestId("wfd-actions");
    const runNow = screen.getByTestId("wfd-run-workflow");
    const demoted = [
      screen.getByTestId("wfd-open-builder"),
      screen.getByTestId("wfd-edit-in-chat"),
      screen.getByTestId("wfd-version-history-link"),
      screen.getByTestId("wfd-add-as-schedule"),
    ];

    expect(actionBar.firstElementChild).toBe(runNow);
    expect(runNow).toHaveClass("wfd-btn-primary");
    expect(runNow).toHaveTextContent("Run now");
    demoted.forEach((action) => expect(action).not.toHaveClass("wfd-btn-primary"));
  });
});
