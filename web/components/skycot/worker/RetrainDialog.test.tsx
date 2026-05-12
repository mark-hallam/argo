import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { RetrainDialog } from "@/components/skycot/worker/RetrainDialog";
import { RecentChangesPanel } from "@/components/skycot/worker/RecentChangesPanel";
import type {
  CommitRetrainResult,
  ClassifyRetrainResult,
  ListRecentRetrainsResult,
  ReverseTestResult,
} from "@/lib/api/retrain";

vi.mock("@/lib/api/retrain", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/retrain")>(
    "@/lib/api/retrain",
  );
  return {
    ...actual,
    classifyRetrain: vi.fn(),
    commitRetrain: vi.fn(),
    listRecentRetrains: vi.fn(),
    undoRetrain: vi.fn(),
    previewReverseTest: vi.fn(),
  };
});

import {
  classifyRetrain,
  commitRetrain,
  listRecentRetrains,
  previewReverseTest,
  undoRetrain,
} from "@/lib/api/retrain";

const mockedClassify = vi.mocked(classifyRetrain);
const mockedCommit = vi.mocked(commitRetrain);
const mockedListRecent = vi.mocked(listRecentRetrains);
const mockedUndo = vi.mocked(undoRetrain);
const mockedReverse = vi.mocked(previewReverseTest);

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
}

function policyProposal(): ClassifyRetrainResult["proposal"] {
  return {
    classification: "policy",
    plainEnglish: "New rule: never ship to PO Boxes. She'll check first.",
    oneLineSummary: 'Policy: never ship to PO Boxes',
    structuralChange: { kind: "policy.update", payload: {} },
  };
}

function withConflictProposal(): ClassifyRetrainResult["proposal"] {
  return {
    ...policyProposal(),
    conflicts: [
      {
        conflictId: "cf_x",
        existingRule: "Use 'customers' in outbound messages.",
        existingRuleAddedAt: new Date(Date.now() - 9 * 24 * 60 * 60_000).toISOString(),
        affectedCaseCount: 6,
      },
    ],
  };
}

function withRegressionProposal(): ClassifyRetrainResult["proposal"] {
  return {
    ...policyProposal(),
    regressionPreview: [
      {
        caseId: "rg_safe",
        context: "Inbound: 'Refund please'",
        before: "Approved.",
        after: "Forwarded to Marta.",
        outcome: "safe",
      },
      {
        caseId: "rg_review",
        context: "Inbound: 'PO Box?'",
        before: "Shipping confirmed.",
        after: "Blocked — PO Box.",
        outcome: "review",
      },
    ],
  };
}

function committedRecord(overrides: Partial<CommitRetrainResult["record"]> = {}) {
  const now = new Date();
  return {
    retrainId: "rt_abc",
    workerId: "wkr_mariah",
    classification: "policy" as const,
    oneLineSummary: "Policy: never ship to PO Boxes",
    committedAt: now.toISOString(),
    undoExpiresAt: new Date(now.getTime() + 14 * 24 * 60 * 60_000).toISOString(),
    ...overrides,
  };
}

function Harness(props: Partial<React.ComponentProps<typeof RetrainDialog>> = {}) {
  const client = makeQueryClient();
  return (
    <QueryClientProvider client={client}>
      <RetrainDialog
        open
        onOpenChange={vi.fn()}
        workerId="wkr_mariah"
        workerName="Mariah"
        triggerContext="charter"
        {...props}
      />
    </QueryClientProvider>
  );
}

function PanelHarness(props: { workerId?: string } = {}) {
  const client = makeQueryClient();
  return (
    <QueryClientProvider client={client}>
      <RecentChangesPanel workerId={props.workerId ?? "wkr_mariah"} />
    </QueryClientProvider>
  );
}

beforeEach(() => {
  mockedClassify.mockResolvedValue({ proposal: policyProposal(), source: "mock" });
  mockedCommit.mockResolvedValue({ record: committedRecord(), source: "mock" });
  mockedListRecent.mockResolvedValue({ records: [], source: "mock" });
  mockedUndo.mockResolvedValue({ ...committedRecord(), undoneAt: new Date().toISOString() });
  mockedReverse.mockResolvedValue({
    cases: [
      { caseId: "rv_1", context: "Inbound: 'PO Box 12 OK?'", before: "Shipping confirmed.", after: "Blocked — PO Box." },
    ],
    source: "mock",
  } satisfies ReverseTestResult);
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("RetrainDialog — step 1 capture", () => {
  it("renders the capture step with a textarea and a disabled Continue button", () => {
    render(<Harness />);
    expect(screen.getByTestId("retrain-step-capture")).toBeInTheDocument();
    expect(screen.getByTestId("retrain-continue")).toBeDisabled();
  });

  it("enables Continue once the textarea has ≥ 10 chars", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const textarea = screen.getByTestId("retrain-input");
    await user.type(textarea, "no PO boxes");
    expect(screen.getByTestId("retrain-continue")).not.toBeDisabled();
  });

  it("cancels and closes the dialog when Cancel is clicked", async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    render(<Harness onOpenChange={onOpenChange} />);
    await user.click(screen.getByTestId("retrain-cancel"));
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});

describe("RetrainDialog — step 2 + 3 classify and propose", () => {
  it("calls the classifier and renders the plain-English proposal", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.type(screen.getByTestId("retrain-input"), "never ship to PO Boxes");
    await user.click(screen.getByTestId("retrain-continue"));
    await waitFor(() => expect(mockedClassify).toHaveBeenCalled());
    expect(await screen.findByTestId("retrain-step-propose")).toBeInTheDocument();
    expect(screen.getByTestId("retrain-plain-english")).toHaveTextContent(/never ship to PO Boxes/);
    expect(screen.getByTestId("retrain-classification-chip")).toHaveTextContent(/Policy/);
  });

  it("opens the Try-it-on-3-cases samples panel when toggled", async () => {
    const user = userEvent.setup();
    mockedClassify.mockResolvedValue({
      proposal: {
        ...policyProposal(),
        trySamples: [
          { caseId: "tr_1", context: "Inbound: 'Quick q'", newResponse: "Applies new rule." },
        ],
      },
      source: "mock",
    });
    render(<Harness />);
    await user.type(screen.getByTestId("retrain-input"), "never ship to PO Boxes");
    await user.click(screen.getByTestId("retrain-continue"));
    await screen.findByTestId("retrain-step-propose");
    await user.click(screen.getByTestId("retrain-try-samples"));
    expect(screen.getByTestId("retrain-samples-panel")).toBeInTheDocument();
    expect(screen.getByTestId("retrain-sample-tr_1")).toHaveTextContent(/Applies new rule/);
  });

  it("surfaces a classify error with retry", async () => {
    const user = userEvent.setup();
    mockedClassify.mockRejectedValueOnce(new Error("offline"));
    render(<Harness />);
    await user.type(screen.getByTestId("retrain-input"), "never ship to PO Boxes");
    await user.click(screen.getByTestId("retrain-continue"));
    expect(await screen.findByTestId("retrain-step-classify-error")).toHaveTextContent(/offline/);
    mockedClassify.mockResolvedValueOnce({ proposal: policyProposal(), source: "mock" });
    await user.click(screen.getByTestId("retrain-classify-retry"));
    await waitFor(() => expect(screen.getByTestId("retrain-step-propose")).toBeInTheDocument());
  });
});

describe("RetrainDialog — step 4 conflict resolution", () => {
  it("shows the conflict step when the proposal carries a conflict", async () => {
    const user = userEvent.setup();
    mockedClassify.mockResolvedValue({ proposal: withConflictProposal(), source: "mock" });
    render(<Harness />);
    await user.type(screen.getByTestId("retrain-input"), "never ship to PO Boxes");
    await user.click(screen.getByTestId("retrain-continue"));
    await screen.findByTestId("retrain-step-propose");
    await user.click(screen.getByTestId("retrain-continue"));
    expect(await screen.findByTestId("retrain-step-conflict")).toBeInTheDocument();
    expect(screen.getByTestId("retrain-conflict-existing")).toHaveTextContent(/customers/);
  });

  it("advances past conflict on Replace and lands on commit when no regressions", async () => {
    const user = userEvent.setup();
    mockedClassify.mockResolvedValue({ proposal: withConflictProposal(), source: "mock" });
    render(<Harness />);
    await user.type(screen.getByTestId("retrain-input"), "never ship to PO Boxes");
    await user.click(screen.getByTestId("retrain-continue"));
    await screen.findByTestId("retrain-step-propose");
    await user.click(screen.getByTestId("retrain-continue"));
    await screen.findByTestId("retrain-step-conflict");
    await user.click(screen.getByTestId("retrain-continue"));
    expect(await screen.findByTestId("retrain-step-commit")).toBeInTheDocument();
  });
});

describe("RetrainDialog — step 5 regression preview", () => {
  it("shows the regression rows with safe/review/blocked outcome tags", async () => {
    const user = userEvent.setup();
    mockedClassify.mockResolvedValue({ proposal: withRegressionProposal(), source: "mock" });
    render(<Harness />);
    await user.type(screen.getByTestId("retrain-input"), "never ship to PO Boxes");
    await user.click(screen.getByTestId("retrain-continue"));
    await screen.findByTestId("retrain-step-propose");
    await user.click(screen.getByTestId("retrain-continue"));
    expect(await screen.findByTestId("retrain-step-regression")).toBeInTheDocument();
    expect(screen.getByTestId("retrain-regression-outcome-rg_safe")).toHaveTextContent(/safe/);
    expect(screen.getByTestId("retrain-regression-outcome-rg_review")).toHaveTextContent(/review/);
  });

  it("pre-selects safe rows and lets the user toggle review rows in", async () => {
    const user = userEvent.setup();
    mockedClassify.mockResolvedValue({ proposal: withRegressionProposal(), source: "mock" });
    render(<Harness />);
    await user.type(screen.getByTestId("retrain-input"), "never ship to PO Boxes");
    await user.click(screen.getByTestId("retrain-continue"));
    await screen.findByTestId("retrain-step-propose");
    await user.click(screen.getByTestId("retrain-continue"));
    await screen.findByTestId("retrain-step-regression");
    const safeRow = screen.getByTestId("retrain-regression-row-rg_safe");
    expect(safeRow).toHaveAttribute("data-accepted", "true");
    const reviewToggle = screen.getByTestId("retrain-regression-accept-rg_review");
    await user.click(reviewToggle);
    const reviewRow = screen.getByTestId("retrain-regression-row-rg_review");
    expect(reviewRow).toHaveAttribute("data-accepted", "true");
  });
});

describe("RetrainDialog — step 6 commit", () => {
  it("fires commit with the proposal and renders the success summary", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.type(screen.getByTestId("retrain-input"), "never ship to PO Boxes");
    await user.click(screen.getByTestId("retrain-continue"));
    await screen.findByTestId("retrain-step-propose");
    await user.click(screen.getByTestId("retrain-continue"));
    expect(await screen.findByTestId("retrain-step-commit")).toBeInTheDocument();
    await user.click(screen.getByTestId("retrain-commit"));
    await waitFor(() =>
      expect(mockedCommit).toHaveBeenCalledWith(
        expect.objectContaining({ workerId: "wkr_mariah" }),
      ),
    );
    expect(await screen.findByTestId("retrain-step-committed")).toBeInTheDocument();
    expect(screen.getByTestId("retrain-committed-summary")).toHaveTextContent(
      /never ship to PO Boxes/i,
    );
  });

  it("opens the reverse-test panel and renders previewed cases", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.type(screen.getByTestId("retrain-input"), "never ship to PO Boxes");
    await user.click(screen.getByTestId("retrain-continue"));
    await screen.findByTestId("retrain-step-propose");
    await user.click(screen.getByTestId("retrain-continue"));
    await screen.findByTestId("retrain-step-commit");
    await user.click(screen.getByTestId("retrain-commit"));
    await screen.findByTestId("retrain-step-committed");
    await user.click(screen.getByTestId("retrain-reverse-test-show"));
    await waitFor(() => expect(mockedReverse).toHaveBeenCalled());
    expect(await screen.findByTestId("retrain-reverse-row-rv_1")).toBeInTheDocument();
  });
});

describe("RetrainDialog — end-to-end happy path", () => {
  it("walks from capture through commit when no conflicts or regressions", async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    render(<Harness onOpenChange={onOpenChange} />);
    await user.type(screen.getByTestId("retrain-input"), "no PO Box shipping");
    await user.click(screen.getByTestId("retrain-continue"));
    await screen.findByTestId("retrain-step-propose");
    await user.click(screen.getByTestId("retrain-continue"));
    expect(await screen.findByTestId("retrain-step-commit")).toBeInTheDocument();
    await user.click(screen.getByTestId("retrain-commit"));
    await screen.findByTestId("retrain-step-committed");
    await user.click(screen.getByTestId("retrain-close"));
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});

describe("RecentChangesPanel", () => {
  it("renders the empty state when no retrains exist", async () => {
    mockedListRecent.mockResolvedValue({ records: [], source: "mock" } satisfies ListRecentRetrainsResult);
    render(<PanelHarness />);
    expect(await screen.findByTestId("recent-changes-empty")).toBeInTheDocument();
  });

  it("renders one row per retrain with relative time and a working Undo button", async () => {
    const user = userEvent.setup();
    mockedListRecent.mockResolvedValue({
      records: [
        committedRecord({ retrainId: "rt_1", oneLineSummary: "Memory: use 'guests'" }),
        committedRecord({ retrainId: "rt_2", oneLineSummary: "Policy: no PO Box shipping" }),
      ],
      source: "mock",
    });
    render(<PanelHarness />);
    expect(await screen.findByTestId("recent-changes-row-rt_1")).toBeInTheDocument();
    expect(screen.getByTestId("recent-changes-row-rt_2")).toHaveTextContent(/no PO Box/);
    await user.click(screen.getByTestId("recent-changes-undo-rt_1"));
    await user.click(await screen.findByTestId("alert-dialog-confirm"));
    await waitFor(() => expect(mockedUndo).toHaveBeenCalledWith("wkr_mariah", "rt_1"));
  });

  it("strikes through entries that have already been undone", async () => {
    mockedListRecent.mockResolvedValue({
      records: [
        {
          ...committedRecord({ retrainId: "rt_old" }),
          undoneAt: new Date().toISOString(),
        },
      ],
      source: "mock",
    });
    render(<PanelHarness />);
    const row = await screen.findByTestId("recent-changes-row-rt_old");
    expect(row).toHaveAttribute("data-undone", "true");
    expect(screen.queryByTestId("recent-changes-undo-rt_old")).toBeNull();
  });
});
