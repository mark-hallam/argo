import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { AuditDetail } from "@/lib/types/audit-detail";

const pushMock = vi.fn();
const backMock = vi.fn();
let routeActionId = "act_priya_001";

vi.mock("next/navigation", () => ({
  useParams: () => ({ actionId: routeActionId }),
  useRouter: () => ({
    push: pushMock,
    back: backMock,
    replace: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    forward: vi.fn(),
  }),
  usePathname: () => `/audit/action/${routeActionId}`,
  useSearchParams: () => new URLSearchParams(),
}));

vi.mock("@/lib/hooks/use-redaction-policy", async () => {
  const { POLICY_SHOW_ALL } = await import("@/lib/redaction/redact");
  return { useRedactionPolicy: () => POLICY_SHOW_ALL };
});

vi.mock("@/lib/api/audit-detail", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/audit-detail")>(
    "@/lib/api/audit-detail",
  );
  return {
    ...actual,
    fetchAuditDetail: vi.fn(),
  };
});

import { AuditDetailNotFoundError, fetchAuditDetail } from "@/lib/api/audit-detail";
import AuditActionDetailPage from "@/app/(app)/audit/action/[actionId]/page";

const mockedFetchAuditDetail = vi.mocked(fetchAuditDetail);

function sampleDetail(overrides: Partial<AuditDetail> = {}): AuditDetail {
  const ts = new Date("2026-05-11T14:32:11Z").toISOString();
  return {
    id: "act_priya_001",
    timestamp: ts,
    worker: { id: "wkr_01J0PRIYA", name: "Priya Mehra" },
    actionType: "candidate_screened",
    verb: "screened candidate Diego Alvarez (diego@example.com) against JD-2024-08",
    status: "succeeded",
    approver: { id: "usr_op_mark", name: "Mark" },
    channel: "slack",
    cost: { usd: 0.014 },
    dataClassification: "confidential",
    inputs: {
      triggerKind: "message",
      triggerSummary: "Application received from Diego Alvarez via Greenhouse webhook.",
      predecessor: { id: "act_priya_000", verb: "received Greenhouse webhook for JD-2024-08" },
    },
    decisionChain: [
      { n: 1, title: "Memory loaded", body: "must-have skills · candidate-prefs (cached)", details: { hashes: ["4f3a9c"] } },
      { n: 2, title: "Skill: recruiter-screen v4", body: "trust 94%" },
      { n: 3, title: "Plan generated", body: "Match 6/6 must-haves." },
      { n: 4, title: "Plan selected", body: "score 0.93" },
      { n: 5, title: "Policy: PASS", body: "no flags" },
      { n: 6, title: "Approval", body: "Autonomous" },
      { n: 7, title: "Execution", body: "Wrote score to Greenhouse" },
      { n: 8, title: "Outcome", body: "Sent, 200 OK" },
    ],
    provenance: {
      model: { id: "claude-opus-4-7", version: "2026-04-30" },
      promptTemplate: { id: "recruiter.screen", hash: "8a3b9f1c2d4e5f6a" },
      skill: { id: "recruiter-screen", version: "4.2.0" },
      memorySnapshot: { id: "mem_4f3a9c12" },
      policy: { id: "recruiter-screen-policy", version: "2.0", hash: "c91e2b4f5a6d7c8e" },
    },
    output: {
      kind: "text",
      body:
        "Decision posted. Reference: https://github.com/acmeco/ops/issues/2912\nAlso https://dashboard.stripe.com/customers/cus_abc123",
      delivery: { sentAt: ts, downstreamId: "gh_ev_4f1c" },
      followUp: { id: "act_priya_002", verb: "booked Thursday 10:30 interview" },
    },
    ...overrides,
  };
}

function PageWrapper() {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0, retryDelay: 0 },
    },
  });
  return (
    <QueryClientProvider client={client}>
      <AuditActionDetailPage />
    </QueryClientProvider>
  );
}

const writeText = vi.fn(async (_text: string) => undefined);

function installClipboardSpy() {
  // userEvent.setup() can shadow navigator.clipboard with its own polyfill;
  // re-install our spy on every test so the click handlers' direct
  // `navigator.clipboard.writeText` calls land on the vitest mock.
  Object.defineProperty(globalThis.navigator, "clipboard", {
    configurable: true,
    writable: true,
    value: { writeText },
  });
}

installClipboardSpy();

beforeEach(() => {
  routeActionId = "act_priya_001";
  pushMock.mockClear();
  backMock.mockClear();
  writeText.mockClear();
  mockedFetchAuditDetail.mockReset();
  installClipboardSpy();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("AuditActionDetailPage", () => {
  it("renders the breadcrumb, summary hero, and all 4 detail cards", async () => {
    mockedFetchAuditDetail.mockResolvedValue(sampleDetail());
    render(<PageWrapper />);
    expect(await screen.findByTestId("audit-detail-page")).toBeInTheDocument();
    expect(screen.getByText("Audit")).toBeInTheDocument();
    expect(screen.getByTestId("audit-summary-hero")).toBeInTheDocument();
    expect(screen.getByTestId("audit-inputs-card")).toBeInTheDocument();
    expect(screen.getByTestId("audit-decision-chain")).toBeInTheDocument();
    expect(screen.getByTestId("audit-provenance-card")).toBeInTheDocument();
    expect(screen.getByTestId("audit-output-card")).toBeInTheDocument();
  });

  it("Replay button routes to /audit/action/<id>/replay", async () => {
    const user = userEvent.setup();
    mockedFetchAuditDetail.mockResolvedValue(sampleDetail());
    render(<PageWrapper />);
    await screen.findByTestId("audit-detail-page");
    await user.click(screen.getByTestId("audit-hero-replay"));
    expect(pushMock).toHaveBeenCalledWith(
      "/audit/action/act_priya_001/replay",
    );
  });

  it("Export button opens the EvidenceBundleExporter in single-action mode", async () => {
    const user = userEvent.setup();
    mockedFetchAuditDetail.mockResolvedValue(sampleDetail());
    render(<PageWrapper />);
    await screen.findByTestId("audit-detail-page");
    await user.click(screen.getByTestId("audit-hero-export"));
    const dialog = await screen.findByTestId("export-dialog");
    expect(dialog).toHaveAttribute("data-mode", "single-action");
    expect(screen.getByTestId("export-scope-locked")).toBeInTheDocument();
  });

  it("renders <AuditActionNotFound> on AuditDetailNotFoundError", async () => {
    routeActionId = "act_missing";
    mockedFetchAuditDetail.mockRejectedValue(new AuditDetailNotFoundError("act_missing"));
    render(<PageWrapper />);
    expect(await screen.findByTestId("audit-action-not-found")).toBeInTheDocument();
    expect(screen.getByText(/act_missing/)).toBeInTheDocument();
  });

  it("renders all 8 decision-chain steps in numeric order", async () => {
    mockedFetchAuditDetail.mockResolvedValue(sampleDetail());
    render(<PageWrapper />);
    await screen.findByTestId("audit-detail-page");
    const chain = screen.getByTestId("audit-decision-chain");
    for (let n = 1; n <= 8; n++) {
      expect(within(chain).getByTestId(`audit-decision-step-${n}`)).toBeInTheDocument();
    }
  });

  it("expands a decision-chain step's details when its toggle is clicked", async () => {
    const user = userEvent.setup();
    mockedFetchAuditDetail.mockResolvedValue(sampleDetail());
    render(<PageWrapper />);
    await screen.findByTestId("audit-detail-page");
    expect(screen.queryByTestId("audit-decision-step-1-details")).toBeNull();
    await user.click(screen.getByTestId("audit-decision-step-1-toggle"));
    expect(screen.getByTestId("audit-decision-step-1-details")).toHaveTextContent(/4f3a9c/);
  });

  it("renders 5 provenance rows with short hashes and a Copy hash chain button", async () => {
    mockedFetchAuditDetail.mockResolvedValue(sampleDetail());
    render(<PageWrapper />);
    await screen.findByTestId("audit-detail-page");
    for (const key of ["model", "prompt", "skill", "memory", "policy"]) {
      expect(screen.getByTestId(`audit-provenance-row-${key}`)).toBeInTheDocument();
    }
    expect(screen.getByTestId("audit-provenance-hash-prompt")).toHaveTextContent(/8a3b9f1c/);
    // fireEvent — userEvent.setup() installs its own clipboard polyfill that
    // shadows the navigator.clipboard.writeText spy we install in beforeEach.
    await act(async () => {
      fireEvent.click(screen.getByTestId("audit-provenance-copy-chain"));
    });
    await waitFor(() => {
      expect(writeText).toHaveBeenCalledTimes(1);
    });
    const firstCall = writeText.mock.calls[0];
    const payload = JSON.parse(firstCall[0]);
    expect(payload).toMatchObject({
      actionId: "act_priya_001",
      model: { id: "claude-opus-4-7" },
      policy: { hash: "c91e2b4f5a6d7c8e" },
    });
  });

  it("copies the action id when the hero copy button is clicked", async () => {
    mockedFetchAuditDetail.mockResolvedValue(sampleDetail());
    render(<PageWrapper />);
    await screen.findByTestId("audit-detail-page");
    await act(async () => {
      fireEvent.click(screen.getByTestId("audit-hero-copy-id"));
    });
    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith("act_priya_001");
    });
  });

  it("renders cross-system jump links for Stripe + GitHub URLs in text output", async () => {
    mockedFetchAuditDetail.mockResolvedValue(sampleDetail());
    render(<PageWrapper />);
    await screen.findByTestId("audit-detail-page");
    const links = screen.getByTestId("audit-output-cross-links");
    expect(within(links).getByTestId("audit-output-cross-link-stripe")).toHaveAttribute(
      "href",
      "https://dashboard.stripe.com/customers/cus_abc123",
    );
    expect(within(links).getByTestId("audit-output-cross-link-github")).toHaveAttribute(
      "href",
      "https://github.com/acmeco/ops/issues/2912",
    );
  });

  it("renders the predecessor link to its own drill-in route", async () => {
    mockedFetchAuditDetail.mockResolvedValue(sampleDetail());
    render(<PageWrapper />);
    await screen.findByTestId("audit-detail-page");
    const pred = screen.getByTestId("audit-inputs-predecessor");
    expect(pred).toHaveAttribute("href", "/audit/action/act_priya_000");
  });

  it("renders the follow-up link in the output card", async () => {
    mockedFetchAuditDetail.mockResolvedValue(sampleDetail());
    render(<PageWrapper />);
    await screen.findByTestId("audit-detail-page");
    expect(screen.getByTestId("audit-output-followup")).toHaveAttribute(
      "href",
      "/audit/action/act_priya_002",
    );
  });

  it("renders the computer-use screenshot trace when output.kind is computer-use", async () => {
    mockedFetchAuditDetail.mockResolvedValue(
      sampleDetail({
        output: {
          kind: "computer-use",
          body: "Browser executor: open page → fill form → submit",
          screenshots: [
            { url: "https://example.com/s1.png", timestamp: "2026-05-11T14:32:09Z", description: "fill form" },
            { url: "https://example.com/s2.png", timestamp: "2026-05-11T14:32:10Z", description: "submit" },
          ],
        },
      }),
    );
    render(<PageWrapper />);
    await screen.findByTestId("audit-detail-page");
    expect(screen.getByTestId("audit-output-card")).toHaveAttribute(
      "data-output-kind",
      "computer-use",
    );
    const trace = screen.getByTestId("audit-output-screenshots");
    expect(trace).toBeInTheDocument();
    expect(within(trace).getByTestId("audit-output-screenshot-0")).toBeInTheDocument();
    expect(within(trace).getByTestId("audit-output-screenshot-1")).toBeInTheDocument();
  });

  it("Back button calls router.back()", async () => {
    const user = userEvent.setup();
    mockedFetchAuditDetail.mockResolvedValue(sampleDetail());
    render(<PageWrapper />);
    await screen.findByTestId("audit-detail-page");
    await user.click(screen.getByTestId("audit-detail-back"));
    expect(backMock).toHaveBeenCalledTimes(1);
  });

  it("renders an error state with retry that calls refetch on click", async () => {
    const user = userEvent.setup();
    mockedFetchAuditDetail.mockRejectedValue(new Error("boom"));
    render(<PageWrapper />);
    // useAuditDetail retries twice on generic errors; default findBy timeout
    // (~1s) isn't enough to span the retry delays, so widen it here.
    const err = await screen.findByTestId(
      "audit-detail-error",
      {},
      { timeout: 5000 },
    );
    expect(err).toHaveTextContent(/Couldn't load action/);
    expect(err).toHaveTextContent(/boom/);
    mockedFetchAuditDetail.mockResolvedValueOnce(sampleDetail());
    await user.click(screen.getByTestId("audit-detail-retry"));
    await waitFor(() => {
      expect(screen.queryByTestId("audit-detail-page")).toBeInTheDocument();
    });
  });
});
