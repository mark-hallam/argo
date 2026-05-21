import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { WorkflowProposeModal } from "./WorkflowProposeModal";
import type { Worker } from "@/lib/types/worker";

const toastSuccessMock = vi.fn();
vi.mock("@/lib/ui/use-toast", () => ({
  useToast: () => ({
    success: toastSuccessMock,
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    promise: vi.fn(),
    dismiss: vi.fn(),
  }),
}));

const SAMPLE_TOML = `schema_version = "1"

[workflow]
name        = "weekly_feedback_triage"
description = "Test workflow."
`;

function stringStream(body: string): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  return new ReadableStream<Uint8Array>({
    start(controller) {
      controller.enqueue(encoder.encode(body));
      controller.close();
    },
  });
}

const WORKERS: Worker[] = [
  {
    id: "wkr_mariah",
    name: "Mariah",
    status: "working",
    role: { tagline: "lists and re-prices items", vertical: "marketplace" },
    headline: { bullets: [] },
  },
  {
    id: "wkr_priya",
    name: "Priya",
    status: "idle",
    role: { tagline: "screens candidates", vertical: "recruiting" },
    headline: { bullets: [] },
  },
];

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
}

function renderModal(extra: Partial<React.ComponentProps<typeof WorkflowProposeModal>> = {}) {
  const queryClient = makeQueryClient();
  return render(
    <QueryClientProvider client={queryClient}>
      <WorkflowProposeModal
        open
        onOpenChange={extra.onOpenChange ?? vi.fn()}
        workersOverride={WORKERS}
        {...extra}
      />
    </QueryClientProvider>,
  );
}

describe("WorkflowProposeModal", () => {
  it("renders worker + vertical pickers, defaulting to the first worker", () => {
    renderModal();
    const workerSelect = screen.getByTestId("workflow-propose-worker") as HTMLSelectElement;
    const verticalSelect = screen.getByTestId("workflow-propose-vertical") as HTMLSelectElement;
    expect(workerSelect.value).toBe("wkr_mariah");
    expect(verticalSelect.value).toBe("marketplace");
  });

  it("switching the worker auto-updates the vertical picker", async () => {
    const user = userEvent.setup();
    renderModal();
    await user.selectOptions(screen.getByTestId("workflow-propose-worker"), "wkr_priya");
    const verticalSelect = screen.getByTestId("workflow-propose-vertical") as HTMLSelectElement;
    expect(verticalSelect.value).toBe("recruiting");
  });

  it("close button fires onOpenChange(false)", async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    renderModal({ onOpenChange });
    await user.click(screen.getByTestId("workflow-propose-close"));
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("editingWorkflow pins the vertical and shifts the title", () => {
    renderModal({
      editingWorkflow: { workflowName: "post_listing", verticalId: "marketplace" },
    });
    expect(screen.getByTestId("workflow-propose-title")).toHaveTextContent(
      /Edit post_listing in chat/,
    );
    const verticalSelect = screen.getByTestId("workflow-propose-vertical") as HTMLSelectElement;
    expect(verticalSelect.value).toBe("marketplace");
    expect(verticalSelect).toBeDisabled();
  });

  it("approval closes the modal and surfaces a success toast", async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    toastSuccessMock.mockClear();
    const fetchImpl = ((input: RequestInfo | URL) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.startsWith("/api/workflows/propose")) {
        return Promise.resolve(
          new Response(stringStream(SAMPLE_TOML), {
            status: 200,
            headers: { "Content-Type": "text/plain" },
          }),
        );
      }
      if (url.startsWith("/api/v1/verticals/")) {
        return Promise.resolve(new Response("{}", { status: 201 }));
      }
      return Promise.reject(new Error(`unstubbed fetch: ${url}`));
    }) as unknown as typeof fetch;

    renderModal({ onOpenChange, fetchImpl });

    await user.type(
      screen.getByLabelText("Describe the responsibility in plain English"),
      "weekly triage",
    );
    await user.click(screen.getByTestId("propose-workflow-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("propose-workflow-approve")).toBeEnabled();
    });
    await user.click(screen.getByTestId("propose-workflow-approve"));

    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "Proposed workflow lands when reviewed",
      expect.objectContaining({
        description: expect.stringContaining("weekly_feedback_triage"),
      }),
    );
  });

  it("renders no-workers fallback when the team is empty", () => {
    renderModal({ workersOverride: [] });
    expect(screen.getByTestId("workflow-propose-no-workers")).toBeInTheDocument();
  });
});
