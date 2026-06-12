/**
 * Connect-on-demand wiring on WorkerOverviewContent.
 *
 * The locked `G2 · Elevated` grant card is fed by the LIVE tg1 endpoint
 * (`GET .../capability-requests`) — polled on mount + after each run —
 * NOT by a field the run route never returns. Behavior pins:
 *
 *  - a pending TOOL request (file family) renders the card, and Connect
 *    posts the durable tg3 grant carrying the tg1 request id;
 *  - a pending CONNECTOR request renders the card, and Connect keeps the
 *    tg2 connector-authorize path (NOT the tg3 grant);
 *  - no pending request → no card (would FAIL/false-hide before this
 *    wiring, since the dead `result.capability_request` was always
 *    undefined);
 *  - the whole surface is mock-gated: a session without a real cookie
 *    never fetches and never shows the card.
 */

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { WorkerOverviewContent } from "./WorkerOverviewContent";

const {
  fetchPendingMock,
  grantCapabilityMock,
  authorizeConnectorScopesMock,
  hasRealSessionMock,
} = vi.hoisted(() => ({
  fetchPendingMock: vi.fn(),
  grantCapabilityMock: vi.fn(),
  authorizeConnectorScopesMock: vi.fn(),
  hasRealSessionMock: vi.fn(() => true),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
    info: vi.fn(),
    loading: vi.fn(),
    promise: vi.fn(),
    dismiss: vi.fn(),
  },
}));

// Keep the pure mappers real (`capabilityRequestToGrant`,
// `isConnectorRequest`); only the two network functions are stubbed.
vi.mock("@/lib/api/capability-requests", async (importActual) => {
  const actual =
    await importActual<typeof import("@/lib/api/capability-requests")>();
  return {
    ...actual,
    fetchPendingCapabilityRequests: fetchPendingMock,
    grantCapability: grantCapabilityMock,
  };
});

vi.mock("@/lib/api/connectors", () => ({
  authorizeConnectorScopes: authorizeConnectorScopesMock,
}));

vi.mock("@/lib/api/workers", () => ({
  triggerWorkerRun: vi.fn(),
}));

vi.mock("@/lib/skycot/demo-mode", () => ({
  hasRealSession: hasRealSessionMock,
}));

vi.mock("@/lib/hooks/use-worker", () => ({
  useWorker: () => ({ data: { id: "w-1", name: "Nora Patel" } }),
}));

vi.mock("@/lib/hooks/use-worker-overview", () => ({
  useWorkerOverview: () => ({
    data: {
      overview: {
        charter: "Nora keeps the pipeline moving.",
        duties: [],
        kpis: [],
        recentActivity: [],
        genesis_plan_id: null,
      },
      source: "gateway",
    },
    isPending: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
  }),
}));

vi.mock("@/lib/hooks/use-duties", () => ({
  useToggleDuty: () => ({ mutate: vi.fn() }),
}));

// Sibling cards are out of scope.
vi.mock("@/components/skycot/worker/WhatTheyDoCard", () => ({
  WhatTheyDoCard: () => null,
  WhatTheyDoCardSkeleton: () => null,
}));
vi.mock("@/components/skycot/worker/DutiesCard", () => ({
  DutiesCard: () => null,
  DutiesCardSkeleton: () => null,
}));
vi.mock("@/components/skycot/worker/ChannelStripCard", () => ({
  ChannelStripCard: () => null,
  ChannelStripCardSkeleton: () => null,
}));
vi.mock("@/components/skycot/worker/KpiCard", () => ({
  KpiCard: () => null,
  KpiCardSkeleton: () => null,
}));
vi.mock("@/components/skycot/worker/RecentActivityCard", () => ({
  RecentActivityCard: () => null,
  RecentActivityCardSkeleton: () => null,
}));
vi.mock("@/components/skycot/worker/WorkerCostChip", () => ({
  WorkerCostChip: () => null,
}));
vi.mock("@/components/skycot/worker/EditDutyDialog", () => ({
  EditDutyDialog: () => null,
}));

function row(over: Record<string, unknown> = {}) {
  return {
    id: "req-1",
    tool_or_connector: "file_read",
    requested_access: "read",
    run_id: null,
    status: "pending",
    created_utc: "2026-06-12T00:00:00Z",
    last_seen_utc: "2026-06-12T00:00:00Z",
    ...over,
  };
}

describe("WorkerOverviewContent — connect-on-demand grant card", () => {
  beforeEach(() => {
    sessionStorage.clear();
    fetchPendingMock.mockReset();
    grantCapabilityMock.mockReset();
    authorizeConnectorScopesMock.mockReset();
    hasRealSessionMock.mockReset();
    hasRealSessionMock.mockReturnValue(true);
    grantCapabilityMock.mockResolvedValue({
      worker_id: "w-1",
      tool: "file_read",
      access: "read",
      outcome: "granted",
    });
    authorizeConnectorScopesMock.mockResolvedValue({
      workerId: "w-1",
      connectorId: "slack",
      ok: true,
    });
  });

  it("renders the card for the newest pending TOOL request and Connect posts the durable tg3 grant", async () => {
    fetchPendingMock.mockResolvedValue({
      workerName: "Nora Patel",
      requests: [row()],
    });
    const user = userEvent.setup();
    render(<WorkerOverviewContent workerId="w-1" />);

    const card = await screen.findByTestId("capability-grant-card");
    expect(card).toBeInTheDocument();

    await user.click(screen.getByTestId("capability-grant-connect"));

    await waitFor(() => expect(grantCapabilityMock).toHaveBeenCalledTimes(1));
    expect(grantCapabilityMock).toHaveBeenCalledWith({
      workerId: "w-1",
      toolOrConnector: "file_read",
      access: "read",
      requestId: "req-1",
    });
    // A tool grant never hits the connector-authorize (tg2) path.
    expect(authorizeConnectorScopesMock).not.toHaveBeenCalled();
    // Card clears on success.
    await waitFor(() =>
      expect(screen.queryByTestId("capability-grant-card")).toBeNull(),
    );
  });

  it("keeps the tg2 connector-authorize path for a known CONNECTOR request", async () => {
    fetchPendingMock.mockResolvedValue({
      workerName: "Nora Patel",
      requests: [row({ tool_or_connector: "slack", requested_access: "write" })],
    });
    const user = userEvent.setup();
    render(<WorkerOverviewContent workerId="w-1" />);

    await screen.findByTestId("capability-grant-card");
    await user.click(screen.getByTestId("capability-grant-connect"));

    await waitFor(() =>
      expect(authorizeConnectorScopesMock).toHaveBeenCalledTimes(1),
    );
    expect(authorizeConnectorScopesMock.mock.calls[0]?.[0]).toMatchObject({
      workerId: "w-1",
      connectorId: "slack",
    });
    expect(grantCapabilityMock).not.toHaveBeenCalled();
  });

  it("shows no card when there are no pending requests", async () => {
    fetchPendingMock.mockResolvedValue({ workerName: "Nora Patel", requests: [] });
    render(<WorkerOverviewContent workerId="w-1" />);

    // Let the mount poll settle.
    await waitFor(() => expect(fetchPendingMock).toHaveBeenCalled());
    await waitFor(() =>
      expect(screen.queryByTestId("capability-grant-card")).toBeNull(),
    );
  });

  it("does not fetch or render the card without a real session (mock-gated)", async () => {
    hasRealSessionMock.mockReturnValue(false);
    fetchPendingMock.mockResolvedValue({
      workerName: "Nora Patel",
      requests: [row()],
    });
    render(<WorkerOverviewContent workerId="w-1" />);

    await screen.findByTestId("worker-overview-run-now");
    expect(fetchPendingMock).not.toHaveBeenCalled();
    expect(screen.queryByTestId("capability-grant-card")).toBeNull();
  });
});
