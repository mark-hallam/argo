import { render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
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

  it("renders an em-dash for decision-history metrics when no rows are decided", () => {
    const cells = computeKpiCells([], 0, NOW);
    expect(cells.find((c) => c.testId === "resolved")?.value).toBe("—");
    expect(cells.find((c) => c.testId === "avg-response")?.value).toBe("—");
    expect(cells.find((c) => c.testId === "approval-rate")?.value).toBe("—");
    expect(cells.find((c) => c.testId === "approval-rate")?.rate).toBeUndefined();
  });

  it("derives resolved count, response age, and approval rate from decided rows", () => {
    const cells = computeKpiCells(
      [
        item({
          id: "approved",
          decision: "approve",
          receivedAt: new Date(NOW - 10 * 60_000).toISOString(),
        }),
        item({
          id: "refused",
          decision: "refuse",
          receivedAt: new Date(NOW - 50 * 60_000).toISOString(),
        }),
        item({
          id: "pending",
          decision: null,
          receivedAt: new Date(NOW - 5 * 60_000).toISOString(),
        }),
      ],
      1,
      NOW,
    );
    expect(cells.find((c) => c.testId === "resolved")?.value).toBe("2");
    expect(cells.find((c) => c.testId === "avg-response")?.value).toBe("30m");
    const rate = cells.find((c) => c.testId === "approval-rate");
    expect(rate?.value).toBe("50%");
    expect(rate?.rate?.slices.approved).toBe(0.5);
    expect(rate?.rate?.slices.refused).toBe(0.5);
  });

  it("uses derived approval-rate ring data instead of stale caller fallbacks", () => {
    render(
      <KpiStrip
        cells={computeKpiCells([], 0, NOW)}
        rate={{
          headline: "99%",
          slices: { approved: 0.99, refused: 0.01, expired: 0 },
        }}
      />,
    );
    expect(screen.queryByTestId("approval-rate-ring")).not.toBeInTheDocument();
  });

  it("renders the 7-day approval-rate ring from decision rows", () => {
    render(
      <KpiStrip
        cells={computeKpiCells(
          [
            item({ id: "a", decision: "approve" }),
            item({ id: "b", decision: "refuse" }),
          ],
          0,
          NOW,
        )}
      />,
    );
    expect(screen.getByTestId("approval-rate-ring")).toBeInTheDocument();
    expect(screen.getByTestId("kpi-rate-ring")).toHaveTextContent(/7-day approval rate/);
    expect(screen.getByTestId("approval-rate-ring")).toHaveAttribute("data-approved", "50");
  });

  it("renders an em-dash for $-at-risk when no items have monetary impact", () => {
    const cells = computeKpiCells([item({})], 1, NOW);
    const atRisk = cells.find((c) => c.testId === "at-risk")!;
    expect(atRisk.value).toBe("—");
    expect(atRisk.sub).toMatch(/no monetary impact/);
  });

  it("does not carry the old fixed KPI literals in the component source", () => {
    const source = readFileSync("components/skycot/approvals/KpiStrip.tsx", "utf8");
    expect(source).not.toMatch(/["']42["']|["']8m["']|["']78%["']|0\.78|0\.14|0\.08/);
  });
});
