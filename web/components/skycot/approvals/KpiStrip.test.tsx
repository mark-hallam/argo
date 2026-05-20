import { render, screen } from "@testing-library/react";
import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";
import { KpiStrip, computeKpiCells } from "./KpiStrip";
import type { ApprovalItem } from "@/lib/sample-approvals";

const NOW = new Date("2026-05-21T12:00:00Z").getTime();

function item(over: Partial<ApprovalItem>): ApprovalItem {
  return {
    id: over.id ?? "x",
    worker: { name: "Worker" },
    verb: "wants to",
    detail: "detail",
    consequenceIfYes: "yes",
    consequenceIfNo: "no",
    channel: "slack",
    receivedAt: new Date(NOW - 60_000).toISOString(),
    ...over,
  };
}

describe("KpiStrip", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "matchMedia",
      vi.fn().mockReturnValue({ matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() }),
    );
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders five cells with the design-locked labels", () => {
    render(
      <KpiStrip
        cells={computeKpiCells([], 0)}
      />,
    );
    for (const label of [
      "Pending",
      "Resolved / wk",
      "Avg response",
      "$ at risk",
      "Approval rate",
    ]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
  });

  it("flags the Pending cell with destructive tone when any item is over SLA", () => {
    const items = [
      item({ id: "a", slaExpiresAt: new Date(NOW - 60_000).toISOString() }),
    ];
    const cells = computeKpiCells(items, 1, NOW);
    expect(cells[0].tone).toBe("bad");
    expect(cells[0].sub).toMatch(/over SLA/);
  });

  it("sums dollar-at-risk across pending items", () => {
    const items = [
      item({ id: "a", dollarAtRisk: 1000 }),
      item({ id: "b", dollarAtRisk: 2500 }),
      item({ id: "c" }),
    ];
    const cells = computeKpiCells(items, 3, NOW);
    const atRisk = cells.find((c) => c.testId === "at-risk")!;
    expect(atRisk.value).toMatch(/\$3,500|\$3\.5/);
    expect(atRisk.sub).toMatch(/across 2 pending/);
  });

  it("renders the 7-day approval-rate ring when rate is provided", () => {
    render(
      <KpiStrip
        cells={computeKpiCells([], 0)}
        rate={{
          headline: "78%",
          slices: { approved: 0.78, refused: 0.14, expired: 0.08 },
        }}
      />,
    );
    expect(screen.getByTestId("approval-rate-ring")).toBeInTheDocument();
    expect(screen.getByTestId("kpi-rate-ring")).toHaveTextContent(/7-day approval rate/);
  });

  it("renders an em-dash for $-at-risk when no items have monetary impact", () => {
    const cells = computeKpiCells([item({})], 1, NOW);
    const atRisk = cells.find((c) => c.testId === "at-risk")!;
    expect(atRisk.value).toBe("—");
    expect(atRisk.sub).toMatch(/no monetary impact/);
  });
});
