/**
 * Acceptance test for
 * `fix-workflows-build-vertical-picker-add-marketing-and-compliance`.
 *
 * Pins:
 *   1. The mock-offline fallback grid carries at least 7 cards
 *      covering every cohort dept tab — including the previously
 *      missing Marketing and Compliance entries.
 *   2. When the gateway returns a non-empty `loaded` list, the
 *      picker renders THAT list (not the 7-entry fallback).
 */

import { render, screen, within } from "@testing-library/react";
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

describe("/workflows/build vertical picker", () => {
  beforeEach(() => {
    verticalsMock.useVerticals.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders the 7-entry fallback grid in mock-offline mode with Marketing and Compliance present", () => {
    verticalsMock.useVerticals.mockReturnValue({
      verticals: [],
      source: "mock-offline",
      isLoading: false,
      error: null,
    });

    render(<WorkflowBuildEntryPage />);

    const picker = screen.getByTestId("workflow-build-vertical-picker");
    expect(picker.getAttribute("data-source")).toBe("fallback");

    const cards = within(picker).getAllByRole("link");
    expect(cards.length).toBeGreaterThanOrEqual(7);

    expect(screen.getByTestId("workflow-build-vertical-marketing")).toBeTruthy();
    expect(screen.getByTestId("workflow-build-vertical-compliance")).toBeTruthy();

    // Sanity: original 5 still present so no regression on existing flows.
    for (const id of ["sales", "support", "gtm-engineer", "people-ops", "finance"]) {
      expect(screen.getByTestId(`workflow-build-vertical-${id}`)).toBeTruthy();
    }
  });

  it("renders the gateway list when the gateway returns loaded verticals (not the fallback)", () => {
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
      {
        id: "pitch-agent",
        name: "Pitch Agent",
        version: "0.1.0",
        description: "Pitches Skycot to investors.",
        headline: "Pitches Skycot to investors.",
        tags: ["pitch"],
        workflow_count: 0,
        action_count: 5,
        role_label: "Pitch Agent",
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

    const picker = screen.getByTestId("workflow-build-vertical-picker");
    expect(picker.getAttribute("data-source")).toBe("gateway");

    expect(screen.getByTestId("workflow-build-vertical-finance-ar")).toBeTruthy();
    expect(screen.getByTestId("workflow-build-vertical-pitch-agent")).toBeTruthy();

    // The fallback hardcoded entries must NOT appear when the gateway
    // returned a non-empty list — the picker must consume the server
    // source, not duplicate it.
    expect(
      screen.queryByTestId("workflow-build-vertical-marketing"),
    ).toBeNull();
    expect(screen.queryByTestId("workflow-build-vertical-compliance")).toBeNull();
  });

  it("renders a skeleton placeholder while the gateway fetch is in flight", () => {
    verticalsMock.useVerticals.mockReturnValue({
      verticals: [],
      source: "mock-offline",
      isLoading: true,
      error: null,
    });

    render(<WorkflowBuildEntryPage />);

    expect(
      screen.getByTestId("workflow-build-vertical-picker-skeleton"),
    ).toBeTruthy();
    expect(screen.queryByTestId("workflow-build-vertical-picker")).toBeNull();
  });
});
