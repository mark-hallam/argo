import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { ReplayOutcome } from "@/lib/api/replay";

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
  usePathname: () => `/audit/action/${routeActionId}/replay`,
  useSearchParams: () => new URLSearchParams(),
}));

vi.mock("@/lib/api/replay", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/replay")>(
    "@/lib/api/replay",
  );
  return {
    ...actual,
    replayAction: vi.fn(),
  };
});

vi.mock("@/lib/api/audit-detail", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/audit-detail")>(
    "@/lib/api/audit-detail",
  );
  return {
    ...actual,
    fetchAuditDetail: vi.fn().mockRejectedValue(
      new actual.AuditDetailNotFoundError("missing"),
    ),
  };
});

import { replayAction } from "@/lib/api/replay";
import ReplayPage from "@/app/(app)/audit/action/[actionId]/replay/page";

const mockedReplayAction = vi.mocked(replayAction);

function reproducibleOutcome(): ReplayOutcome {
  return {
    kind: "success",
    verdict: "reproducible",
    result: {
      originalOutput: { kind: "text", body: "Score: 0.93\nRecommendation: advance" },
      replayedOutput: { kind: "text", body: "Score: 0.93\nRecommendation: advance" },
      provenanceDiff: {
        model: { changed: false },
        promptTemplate: { changed: false },
        skill: { changed: false },
        memorySnapshot: { changed: false },
        policy: { changed: false },
      },
    },
  };
}

function driftedAlignedOutcome(): ReplayOutcome {
  return {
    kind: "success",
    verdict: "drifted-aligned",
    result: {
      originalOutput: { kind: "text", body: "Hello world" },
      replayedOutput: { kind: "text", body: "Hello brave new world" },
      provenanceDiff: {
        model: { changed: false },
        promptTemplate: { changed: false },
        skill: { changed: true, from: "skill v3.0.2", to: "skill v3.0.4" },
        memorySnapshot: { changed: false },
        policy: { changed: false },
      },
    },
  };
}

function driftedMisalignedOutcome(): ReplayOutcome {
  return {
    kind: "success",
    verdict: "drifted-misaligned",
    result: {
      originalOutput: { kind: "text", body: "Reply: SLA 99.95%." },
      replayedOutput: { kind: "text", body: "Reply: SLA 99.99% guaranteed." },
      provenanceDiff: {
        model: { changed: false },
        promptTemplate: { changed: false },
        skill: { changed: false },
        memorySnapshot: { changed: false },
        policy: { changed: true, from: "policy v3.0", to: "policy v3.1" },
      },
    },
  };
}

function unavailableOutcome(): ReplayOutcome {
  return { kind: "unavailable", reason: "model-decommissioned", modelName: "claude-3-opus" };
}

function failedOutcome(): ReplayOutcome {
  return { kind: "failed", reason: "Sandbox runner crashed (exit 137 — OOM)." };
}

function leakOutcome(): ReplayOutcome {
  return {
    kind: "leak-detected",
    details: "Outbound HTTP call to https://hooks.slack.com was not stubbed.",
  };
}

function PageWrapper() {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0, retryDelay: 0 },
      mutations: { retry: false },
    },
  });
  return (
    <QueryClientProvider client={client}>
      <ReplayPage />
    </QueryClientProvider>
  );
}

beforeEach(() => {
  routeActionId = "act_priya_001";
  pushMock.mockClear();
  backMock.mockClear();
  mockedReplayAction.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("ReplayPage", () => {
  it("renders the sandbox banner and verdict pill on reproducible", async () => {
    mockedReplayAction.mockResolvedValue(reproducibleOutcome());
    render(<PageWrapper />);
    expect(await screen.findByTestId("replay-page")).toBeInTheDocument();
    expect(screen.getByTestId("replay-sandbox-banner")).toBeInTheDocument();
    expect(
      await screen.findByTestId("replay-verdict-pill-reproducible"),
    ).toBeInTheDocument();
    expect(screen.getByTestId("replay-diff")).toBeInTheDocument();
    expect(screen.getByTestId("replay-provenance-diff")).toBeInTheDocument();
  });

  it("renders the warning verdict pill on drifted-aligned with skill axis changed", async () => {
    mockedReplayAction.mockResolvedValue(driftedAlignedOutcome());
    render(<PageWrapper />);
    expect(
      await screen.findByTestId("replay-verdict-pill-drifted-aligned"),
    ).toBeInTheDocument();
    const skillRow = screen.getByTestId("replay-provenance-row-skill");
    expect(skillRow).toHaveAttribute("data-changed", "true");
    expect(screen.getByTestId("replay-provenance-from")).toHaveTextContent("skill v3.0.2");
    expect(screen.getByTestId("replay-provenance-to")).toHaveTextContent("skill v3.0.4");
  });

  it("renders the misaligned banner + destructive verdict on drifted-misaligned", async () => {
    mockedReplayAction.mockResolvedValue(driftedMisalignedOutcome());
    render(<PageWrapper />);
    expect(await screen.findByTestId("replay-misaligned-banner")).toHaveTextContent(
      /Operator notified/,
    );
    expect(
      screen.getByTestId("replay-verdict-pill-drifted-misaligned"),
    ).toBeInTheDocument();
  });

  it("renders the model-decommissioned 'Replay unavailable' fallback", async () => {
    mockedReplayAction.mockResolvedValue(unavailableOutcome());
    render(<PageWrapper />);
    expect(await screen.findByTestId("replay-unavailable")).toHaveTextContent(
      /Replay unavailable/,
    );
    expect(screen.getByTestId("replay-unavailable-reason")).toHaveTextContent(
      /claude-3-opus/,
    );
    // No verdict pill / diff cards when unavailable.
    expect(screen.queryByTestId("replay-diff")).toBeNull();
    expect(screen.queryByTestId("replay-provenance-diff")).toBeNull();
  });

  it("renders the failed fallback with a working retry button", async () => {
    const user = userEvent.setup();
    mockedReplayAction.mockResolvedValue(failedOutcome());
    render(<PageWrapper />);
    expect(await screen.findByTestId("replay-failed")).toBeInTheDocument();
    expect(screen.getByTestId("replay-failed-reason")).toHaveTextContent(/OOM/);
    mockedReplayAction.mockResolvedValueOnce(reproducibleOutcome());
    await user.click(screen.getByTestId("replay-failed-retry"));
    expect(
      await screen.findByTestId("replay-verdict-pill-reproducible"),
    ).toBeInTheDocument();
  });

  it("renders the leak-detected red banner with no diff cards", async () => {
    mockedReplayAction.mockResolvedValue(leakOutcome());
    render(<PageWrapper />);
    const leak = await screen.findByTestId("replay-leak");
    expect(leak).toHaveTextContent(/side-effect leak detected/i);
    expect(screen.getByTestId("replay-leak-details")).toHaveTextContent(
      /hooks\.slack\.com/,
    );
    expect(screen.queryByTestId("replay-diff")).toBeNull();
    expect(screen.queryByTestId("replay-provenance-diff")).toBeNull();
  });

  it("Back button routes to /audit/action/<id>", async () => {
    const user = userEvent.setup();
    mockedReplayAction.mockResolvedValue(reproducibleOutcome());
    render(<PageWrapper />);
    await screen.findByTestId("replay-verdict-pill-reproducible");
    await user.click(screen.getByTestId("replay-back"));
    expect(pushMock).toHaveBeenCalledWith("/audit/action/act_priya_001");
  });

  it("Export button pops the E5-e placeholder toast", async () => {
    const user = userEvent.setup();
    mockedReplayAction.mockResolvedValue(reproducibleOutcome());
    render(<PageWrapper />);
    await screen.findByTestId("replay-verdict-pill-reproducible");
    await user.click(screen.getByTestId("replay-export"));
    expect(screen.getByTestId("replay-toast")).toHaveTextContent(/E5-e/);
  });

  it("renders the loading progress block before the outcome resolves", async () => {
    let resolveOutcome: (o: ReplayOutcome) => void = () => undefined;
    mockedReplayAction.mockImplementation(
      () => new Promise<ReplayOutcome>((res) => {
        resolveOutcome = res;
      }),
    );
    render(<PageWrapper />);
    expect(await screen.findByTestId("replay-progress")).toBeInTheDocument();
    expect(screen.getByTestId("replay-progress-label")).toHaveTextContent(
      /step \d+ of \d+/,
    );
    resolveOutcome(reproducibleOutcome());
    await waitFor(() => {
      expect(screen.queryByTestId("replay-progress")).toBeNull();
    });
  });

  it("synthesises a failed outcome when replayAction throws", async () => {
    mockedReplayAction.mockRejectedValue(new Error("network down"));
    render(<PageWrapper />);
    const failed = await screen.findByTestId("replay-failed");
    expect(failed).toHaveTextContent(/network down/);
  });

  it("token-level diff highlights added + removed tokens on drifted output", async () => {
    mockedReplayAction.mockResolvedValue(driftedAlignedOutcome());
    render(<PageWrapper />);
    await screen.findByTestId("replay-verdict-pill-drifted-aligned");
    expect(screen.getAllByTestId("replay-diff-token-add").length).toBeGreaterThan(0);
  });
});
