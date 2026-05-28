/**
 * Regression pin for
 * `fix-workflows-build-vertical-picker-add-marketplace`.
 *
 * UX-AUDIT-T0015 F7 [P1]: the workflow-builder vertical picker omitted
 * `marketplace`, the vertical the canonical Scene-5 demo workflow
 * `customer_research_followup` is seeded under
 * (`crates/argo-demo-seed/src/workflow.rs:26`). Without a marketplace
 * tile the operator cannot author a new marketplace workflow from the
 * canvas entry page when the gateway returns an empty catalog.
 *
 * Pins:
 *   1. The mock-offline fallback grid renders a `marketplace` tile,
 *      and that tile routes to `/workflows/marketplace/untitled/build`.
 *   2. When the gateway returns a non-empty list of verticals (none of
 *      which is `marketplace`), the picker consumes the gateway list
 *      verbatim — fallback entries do NOT bleed through. This pins
 *      the no-parallel-data-source invariant from the original
 *      brief's rebase note.
 */

import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VerticalRow } from "@/lib/api/verticals";

const verticalsMock = vi.hoisted(() => ({
  useVerticals: vi.fn(() => ({
    verticals: [] as VerticalRow[],
    source: "mock-offline" as "gateway" | "mock-offline" | "mock-bypass",
    isLoading: false,
    error: null as Error | null,
  })),
}));

vi.mock("@/lib/api/verticals", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/api/verticals")>(
      "@/lib/api/verticals",
    );
  return { ...actual, useVerticals: verticalsMock.useVerticals };
});

import WorkflowBuildEntryPage from "../page";

describe("/workflows/build vertical picker — marketplace pin", () => {
  beforeEach(() => {
    verticalsMock.useVerticals.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders a marketplace tile in mock-offline fallback that routes to /workflows/marketplace/untitled/build", () => {
    verticalsMock.useVerticals.mockReturnValue({
      verticals: [],
      source: "mock-offline",
      isLoading: false,
      error: null,
    });

    render(<WorkflowBuildEntryPage />);

    const tile = screen.getByTestId("workflow-build-vertical-marketplace");
    expect(tile).toBeTruthy();
    expect(tile.getAttribute("href")).toBe(
      "/workflows/marketplace/untitled/build",
    );
  });

  it("does not bleed fallback marketplace into gateway-sourced lists", () => {
    const gatewayVerticals: VerticalRow[] = [
      {
        id: "finance-ar",
        name: "Finance — Accounts Receivable",
        version: "0.1.0",
        description: "AR collections.",
        headline: "AR collections.",
        tags: ["finance"],
        workflow_count: 0,
        action_count: 4,
        role_label: "Finance — Accounts Receivable",
        kpi_count: 0,
      },
    ];

    verticalsMock.useVerticals.mockReturnValue({
      verticals: gatewayVerticals,
      source: "gateway",
      isLoading: false,
      error: null,
    });

    render(<WorkflowBuildEntryPage />);

    expect(screen.getByTestId("workflow-build-vertical-finance-ar")).toBeTruthy();
    expect(
      screen.queryByTestId("workflow-build-vertical-marketplace"),
    ).toBeNull();
  });
});
