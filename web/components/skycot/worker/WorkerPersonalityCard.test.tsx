import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { WorkerPersonalityCard } from "@/components/skycot/worker/WorkerPersonalityCard";
import { fetchWorker } from "@/lib/api/workers";
import { DEFAULT_DELEGATOR_PERMISSIONS, type Worker } from "@/lib/types/worker";

vi.mock("@/lib/api/workers", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/api/workers")>("@/lib/api/workers");
  return { ...actual, fetchWorker: vi.fn() };
});

const mockedFetchWorker = vi.mocked(fetchWorker);

function makeWorker(over: Partial<Worker>): Worker {
  return {
    id: "w-test",
    name: "Worker",
    role: { tagline: "does things", vertical: "support" },
    status: "idle",
    permissions: DEFAULT_DELEGATOR_PERMISSIONS,
    ...over,
  };
}

function renderCard(workerId: string) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  });
  return render(
    <QueryClientProvider client={client}>
      <WorkerPersonalityCard workerId={workerId} />
    </QueryClientProvider>,
  );
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("WorkerPersonalityCard", () => {
  it("recovers a canonical persona BY NAME when the gateway uses a fresh UUID (demo-live)", async () => {
    // The live gateway re-seeds Marcus with a UUID that does NOT match
    // the hardcoded id on the SPA persona spine, but his name + the
    // tagline trait-suffix are stable.
    mockedFetchWorker.mockResolvedValue(
      makeWorker({
        id: "019e7dca-d933-70c0-8a8e-77fefbc19824",
        name: "Marcus",
        role: {
          tagline:
            "Marketing: drafts replies in Mark's voice. Traits: On-voice, Listening-first, Truth-bounded, Draft-only.",
          vertical: "marketing-agent",
        },
      }),
    );

    renderCard("019e7dca-d933-70c0-8a8e-77fefbc19824");

    // Real trait chips, not the generic Reliable/Curious/Helpful.
    await waitFor(() => {
      expect(screen.getByText("On-voice")).toBeInTheDocument();
    });
    expect(screen.getByText("Truth-bounded")).toBeInTheDocument();
    expect(screen.getByText("Draft-only")).toBeInTheDocument();
    expect(screen.queryByText("Reliable")).not.toBeInTheDocument();

    // Marcus's REAL escalate-when copy, recovered by name.
    expect(
      screen.getByText(/Files every draft to the approval queue/i),
    ).toBeInTheDocument();
  });

  it("falls back to traits parsed off the tagline for a non-persona seeded worker", async () => {
    mockedFetchWorker.mockResolvedValue(
      makeWorker({
        id: "live-uuid-not-in-registry",
        name: "Devon",
        role: {
          tagline: "Field Ops: schedules site visits. Traits: Punctual, Hands-on, Safety-first.",
          vertical: "field-ops",
        },
      }),
    );

    renderCard("live-uuid-not-in-registry");

    await waitFor(() => {
      expect(screen.getByText("Punctual")).toBeInTheDocument();
    });
    expect(screen.getByText("Hands-on")).toBeInTheDocument();
    expect(screen.getByText("Safety-first")).toBeInTheDocument();
    // No persona match → generic great-at copy is the honest fallback.
    expect(
      screen.getByText(/Working through the schedules you set/i),
    ).toBeInTheDocument();
  });

  it("uses the generic defaults only when there is no persona AND no trait suffix", async () => {
    mockedFetchWorker.mockResolvedValue(
      makeWorker({
        id: "bare-hire",
        name: "Atlas",
        role: { tagline: "an inert smoke worker", vertical: "marketplace" },
      }),
    );

    renderCard("bare-hire");

    await waitFor(() => {
      expect(screen.getByText("Reliable")).toBeInTheDocument();
    });
    expect(screen.getByText("Curious")).toBeInTheDocument();
    expect(screen.getByText("Helpful")).toBeInTheDocument();
  });
});
