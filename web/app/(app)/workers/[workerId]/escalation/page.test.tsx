import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";

// A worker id that exists in the page's SEEDS table (Mariah — has both
// per-worker sample rules AND sample history rows), so the anonymous
// path can assert the sample surfaces render.
const SEEDED_WORKER_ID = "019e6785-928a-7270-a766-f945b6cd7161";

vi.mock("next/navigation", () => ({
  useParams: () => ({ workerId: SEEDED_WORKER_ID }),
}));

// Pin the worker name so the headings render deterministically without
// a real `/api/v1/workers/<id>` round-trip.
vi.mock("@/lib/hooks/use-worker", () => ({
  useWorker: () => ({ data: { name: "Mariah" } }),
}));

// Control the live read. The page wires the escalation RULES onto this
// fetcher; we drive both the live-gateway path and the anonymous-mock
// path through it.
const escMock = vi.hoisted(() => ({ fetchWorkerEscalations: vi.fn() }));
vi.mock("@/lib/api/escalations", () => escMock);

import WorkerEscalationPage from "./page";

function wrapper({ children }: { children: ReactNode }) {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}

// A live condition the gateway returns; deliberately distinct from any
// SEEDS sample condition so we can assert the live value wins.
const GATEWAY_CONDITION = "Refund amount exceeds the live $500 threshold";
// A SEEDS-only Mariah condition that must NOT appear on the live path.
const SAMPLE_ONLY_CONDITION =
  "Price drops below the seller's floor by more than 5%";

afterEach(() => {
  vi.clearAllMocks();
});

describe("WorkerEscalationPage — live escalation-rules wiring (NO THEATER)", () => {
  it("real/demo-live: renders the LIVE rules + honest-empty history, not the sample", async () => {
    escMock.fetchWorkerEscalations.mockResolvedValueOnce({
      policy: {
        rules: [
          {
            action: "Issue a refund",
            condition: GATEWAY_CONDITION,
            routesTo: "Finance",
            channel: "Email",
          },
        ],
        isDefault: false,
        readOnly: false,
      },
      source: "gateway",
    });

    render(<WorkerEscalationPage />, { wrapper });

    // The live condition appears (in both the rules card + the matrix)...
    expect((await screen.findAllByText(GATEWAY_CONDITION)).length).toBeGreaterThan(0);
    // ...and the SEEDS-only sample condition does NOT leak in.
    expect(screen.queryAllByText(SAMPLE_ONLY_CONDITION)).toHaveLength(0);
    // History has no live feed → honest empty state, never fabricated rows.
    expect(screen.getByTestId("escalation-history-empty")).toBeInTheDocument();
    expect(screen.queryByTestId("escalation-history-row")).not.toBeInTheDocument();
  });

  it("anonymous dev: renders the per-worker sample rules + sample history", async () => {
    escMock.fetchWorkerEscalations.mockResolvedValueOnce({
      policy: {
        rules: [
          {
            action: "Re-price a listing",
            condition: SAMPLE_ONLY_CONDITION,
            routesTo: "Seller",
          },
        ],
        isDefault: true,
        readOnly: false,
      },
      source: "mock-bypass",
    });

    render(<WorkerEscalationPage />, { wrapper });

    // The sample rule renders for the anonymous-dev path...
    expect((await screen.findAllByText(SAMPLE_ONLY_CONDITION)).length).toBeGreaterThan(0);
    // ...and the page's per-worker sample history rows render (Mariah seed).
    expect(screen.getAllByTestId("escalation-history-row").length).toBeGreaterThan(0);
    expect(screen.queryByTestId("escalation-history-empty")).not.toBeInTheDocument();
  });
});
