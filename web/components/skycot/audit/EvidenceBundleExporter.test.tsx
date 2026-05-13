import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { EvidenceBundleExporter } from "@/components/skycot/audit/EvidenceBundleExporter";
import type { ExportScope } from "@/lib/sample-export";

vi.mock("@/lib/hooks/use-audit-export", async () => {
  const actual = await vi.importActual<
    typeof import("@/lib/hooks/use-audit-export")
  >("@/lib/hooks/use-audit-export");
  return { ...actual };
});

vi.mock("@/lib/api/audit-export", () => {
  const previewExport = vi.fn();
  const generateExport = vi.fn();
  return {
    previewExport,
    generateExport,
    recordedMockAudit: [] as unknown[],
    resetMockAuditLog: vi.fn(),
  };
});

import { generateExport, previewExport } from "@/lib/api/audit-export";

const mockedPreview = vi.mocked(previewExport);
const mockedGenerate = vi.mocked(generateExport);

function makeQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0, retryDelay: 0 } },
  });
}

interface RenderOpts {
  initialScope?: ExportScope;
  mode?: "bulk" | "single-action" | "replay";
  initialFormat?: "soc2-type-ii" | "gdpr-article-15" | "hipaa-disclosure-log" | "generic";
}

function renderExporter(opts: RenderOpts = {}) {
  const onOpenChange = vi.fn();
  const onToast = vi.fn();
  const queryClient = makeQueryClient();
  const ui = render(
    <QueryClientProvider client={queryClient}>
      <EvidenceBundleExporter
        open={true}
        onOpenChange={onOpenChange}
        tenantId="tenant_acme"
        tenantLabel="ACME Inc."
        initialScope={opts.initialScope ?? { kind: "filtered" }}
        initialFormat={opts.initialFormat}
        mode={opts.mode ?? "bulk"}
        onToast={onToast}
      />
    </QueryClientProvider>,
  );
  return { ...ui, onOpenChange, onToast };
}

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  mockedPreview.mockReset();
  mockedGenerate.mockReset();
  mockedPreview.mockResolvedValue({ count: 2_400, sizeEstimate: 2_400 * 4_000 });
  mockedGenerate.mockResolvedValue({
    kind: "immediate",
    url: "/mock-exports/soc2-type-ii/filtered-2400.zip",
    fileName: "soc2-type-ii-filtered.zip",
    sizeEstimate: 2_400 * 4_000,
  });
});

afterEach(() => {
  vi.useRealTimers();
  vi.clearAllMocks();
});

async function flushPreview() {
  await vi.advanceTimersByTimeAsync(350);
  await waitFor(() =>
    expect(screen.queryByTestId("export-preview-stats")).toBeInTheDocument(),
  );
}

describe("EvidenceBundleExporter", () => {
  it("renders the dialog with title, scope, format, tenant, and Export button", async () => {
    renderExporter();
    expect(screen.getByText("Export evidence bundle")).toBeInTheDocument();
    expect(screen.getByTestId("export-scope-selector")).toBeInTheDocument();
    expect(screen.getByTestId("export-format-selector")).toBeInTheDocument();
    expect(screen.getByTestId("export-dialog-tenant")).toHaveTextContent("ACME Inc.");
    expect(screen.getByTestId("export-dialog-submit")).toBeInTheDocument();
  });

  it("defaults to SOC2 Type II format and shows its description copy", () => {
    renderExporter();
    const option = screen.getByTestId("export-format-option-soc2-type-ii");
    expect(option).toHaveAttribute("data-selected");
    expect(screen.getByTestId("export-format-help")).toHaveTextContent(
      /SOC2 Type II evidence bundle/,
    );
  });

  it("switches format and updates the description copy", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    renderExporter();
    await user.click(screen.getByTestId("export-format-option-hipaa-disclosure-log"));
    expect(screen.getByTestId("export-format-help")).toHaveTextContent(
      /HIPAA disclosure log/,
    );
  });

  it("switching scope to range reveals the from/to date pickers", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    renderExporter();
    await user.click(screen.getByTestId("export-scope-option-range"));
    expect(screen.getByTestId("export-scope-range-from")).toBeInTheDocument();
    expect(screen.getByTestId("export-scope-range-to")).toBeInTheDocument();
  });

  it("switching scope to sampled reveals the sample-size input", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    renderExporter();
    await user.click(screen.getByTestId("export-scope-option-sampled"));
    expect(screen.getByTestId("export-scope-sampled-input")).toBeInTheDocument();
  });

  it("shows the preview stats with action count + estimated size", async () => {
    renderExporter();
    await flushPreview();
    expect(screen.getByTestId("export-preview-action-count")).toHaveTextContent("2,400");
    expect(screen.getByTestId("export-preview-size-estimate")).toHaveTextContent("MB");
  });

  it("immediate path: clicking Export triggers download and closes the dialog", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    const { onToast, onOpenChange } = renderExporter();
    await flushPreview();
    await user.click(screen.getByTestId("export-dialog-submit"));
    await waitFor(() => expect(mockedGenerate).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
    expect(onToast).toHaveBeenCalledWith(expect.stringMatching(/Export ready/));
  });

  it("confirm path: 50-100MB preview surfaces the confirm dialog before download", async () => {
    mockedPreview.mockResolvedValue({ count: 18_000, sizeEstimate: 75 * 1024 * 1024 });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    renderExporter();
    await flushPreview();
    expect(screen.getByTestId("export-preview-stats")).toHaveAttribute(
      "data-path",
      "confirm",
    );
    await user.click(screen.getByTestId("export-dialog-submit"));
    expect(screen.getByTestId("export-dialog-confirm")).toHaveTextContent(/Continue/);
    expect(mockedGenerate).not.toHaveBeenCalled();
    await user.click(screen.getByTestId("export-confirm-continue"));
    await waitFor(() => expect(mockedGenerate).toHaveBeenCalledTimes(1));
  });

  it("queued path: >100MB result replaces dialog body with queued copy + Done button", async () => {
    mockedGenerate.mockResolvedValue({
      kind: "queued",
      eta: new Date(Date.now() + 5 * 60_000).toISOString(),
      sizeEstimate: 250 * 1024 * 1024,
    });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    const { onToast } = renderExporter();
    await flushPreview();
    await user.click(screen.getByTestId("export-dialog-submit"));
    await waitFor(() =>
      expect(screen.getByTestId("export-dialog-queued")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("export-dialog-queued")).toHaveTextContent(
      /We'll email you/,
    );
    expect(onToast).toHaveBeenCalledWith(expect.stringMatching(/queued/i));
    expect(screen.getByTestId("export-dialog-done")).toBeInTheDocument();
  });

  it("single-action mode locks the scope and disables the radio selector", () => {
    renderExporter({
      mode: "single-action",
      initialScope: {
        kind: "single-action",
        actionId: "act_priya_001",
        timestamp: "2026-05-11T14:32:11Z",
      },
    });
    expect(screen.getByTestId("export-dialog")).toHaveAttribute(
      "data-mode",
      "single-action",
    );
    expect(screen.getByTestId("export-scope-locked")).toHaveTextContent(
      /act_priya_001/,
    );
    expect(screen.queryByTestId("export-scope-selector")).toBeNull();
  });

  it("replay mode locks the scope to the replay artifact", () => {
    renderExporter({
      mode: "replay",
      initialScope: { kind: "replay", replayId: "replay_2026" },
    });
    expect(screen.getByTestId("export-scope-locked")).toHaveTextContent(
      /replay_2026/,
    );
  });

  it("Cancel closes the dialog without triggering an export", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    const { onOpenChange } = renderExporter();
    await user.click(screen.getByTestId("export-dialog-cancel"));
    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(mockedGenerate).not.toHaveBeenCalled();
  });

  it("Export failure surfaces an error toast and keeps the dialog open", async () => {
    mockedGenerate.mockRejectedValue(new Error("boom"));
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    const { onToast, onOpenChange } = renderExporter();
    await flushPreview();
    await user.click(screen.getByTestId("export-dialog-submit"));
    await waitFor(() =>
      expect(onToast).toHaveBeenCalledWith(expect.stringMatching(/failed/i)),
    );
    expect(onOpenChange).not.toHaveBeenCalledWith(false);
  });
});
