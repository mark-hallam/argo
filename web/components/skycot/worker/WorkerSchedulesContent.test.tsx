import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WorkerSchedulesContent } from "@/components/skycot/worker/WorkerSchedulesContent";
import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { hasRealSession } from "@/lib/skycot/demo-mode";
import { fetchWorker } from "@/lib/api/workers";
import { findSampleWorkerOverview } from "@/lib/sample-worker-overview";
import { DEFAULT_DELEGATOR_PERMISSIONS, type Worker } from "@/lib/types/worker";

// Mock the transport + the session gate so the REAL `listDuties` logic
// (the NO-THEATER `mockFallbackAllowed` gate) runs under test, rather
// than stubbing `listDuties` itself.
vi.mock("@/lib/api/client", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/client")>(
    "@/lib/api/client",
  );
  return { ...actual, apiGet: vi.fn() };
});
vi.mock("@/lib/skycot/demo-mode", async () => {
  const actual = await vi.importActual<typeof import("@/lib/skycot/demo-mode")>(
    "@/lib/skycot/demo-mode",
  );
  return { ...actual, hasRealSession: vi.fn() };
});
vi.mock("@/lib/api/workers", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/workers")>(
    "@/lib/api/workers",
  );
  return { ...actual, fetchWorker: vi.fn() };
});

const mockedApiGet = vi.mocked(apiGet);
const mockedHasRealSession = vi.mocked(hasRealSession);
const mockedFetchWorker = vi.mocked(fetchWorker);

// Priya — a canonical sample worker; her bundled sample duty label is
// the fixture we assert NEVER leaks onto a real session / demo-live.
const PRIYA_ID = "019e6785-928d-78b1-a6b2-136d9ed263ce";
const SAMPLE_DUTY_LABEL =
  findSampleWorkerOverview(PRIYA_ID)?.duties[0]?.label ?? "";

function sampleWorker(): Worker {
  return {
    id: PRIYA_ID,
    name: "Priya",
    role: { tagline: "Senior Recruiter (AI)", vertical: "recruiter" },
    status: "working",
    permissions: DEFAULT_DELEGATOR_PERMISSIONS,
  };
}

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  });
}

function Wrapper({ workerId = PRIYA_ID }: { workerId?: string }) {
  const client = makeQueryClient();
  return (
    <QueryClientProvider client={client}>
      <WorkerSchedulesContent workerId={workerId} />
    </QueryClientProvider>
  );
}

beforeEach(() => {
  mockedFetchWorker.mockResolvedValue(sampleWorker());
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("WorkerSchedulesContent — live listDuties wire (NO-THEATER)", () => {
  it("renders the LIVE gateway schedules and never the bundled sample on a real session", async () => {
    mockedHasRealSession.mockReturnValue(true);
    mockedApiGet.mockResolvedValue([
      { id: "g1", label: "LIVE gateway schedule", cron: "0 9 * * 1-5", enabled: true },
    ]);

    render(<Wrapper />);

    expect(await screen.findByText("LIVE gateway schedule")).toBeInTheDocument();
    // The fabricated sample must NOT appear once a real session is in play.
    expect(SAMPLE_DUTY_LABEL.length).toBeGreaterThan(0);
    expect(screen.queryByText(SAMPLE_DUTY_LABEL)).not.toBeInTheDocument();
  });

  it("renders the honest empty state for a real session with an empty gateway list", async () => {
    mockedHasRealSession.mockReturnValue(true);
    mockedApiGet.mockResolvedValue([]);

    render(<Wrapper />);

    expect(await screen.findByText(/No schedules yet/i)).toBeInTheDocument();
    expect(screen.queryByText(SAMPLE_DUTY_LABEL)).not.toBeInTheDocument();
  });

  it("falls back to the bundled sample for an anonymous dev on a gateway 404", async () => {
    mockedHasRealSession.mockReturnValue(false);
    mockedApiGet.mockRejectedValue(new ApiError(404, "Not found", "not_found"));

    render(<Wrapper />);

    expect(await screen.findByText(SAMPLE_DUTY_LABEL)).toBeInTheDocument();
  });
});
