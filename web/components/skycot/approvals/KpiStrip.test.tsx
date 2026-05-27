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

  it("renders zero-valued counts and rates instead of em-dash when no rows are decided", () => {
    const cells = computeKpiCells([], 0, NOW);
    expect(cells.find((c) => c.testId === "pending")?.value).toBe("0");
    expect(cells.find((c) => c.testId === "resolved")?.value).toBe("0");
    expect(cells.find((c) => c.testId === "approval-rate")?.value).toBe("0%");
    expect(cells.find((c) => c.testId === "approval-rate")?.rate).toBeUndefined();
  });

  it("keeps the em-dash on Avg response when no rows are decided and adds an aria-description", () => {
    const cells = computeKpiCells([], 0, NOW);
    const avg = cells.find((c) => c.testId === "avg-response")!;
    expect(avg.value).toBe("—");
    expect(avg.ariaDescription).toBe("no resolved approvals yet");
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

  it("renders $0 for $-at-risk when no items have monetary impact", () => {
    const cells = computeKpiCells([item({})], 1, NOW);
    const atRisk = cells.find((c) => c.testId === "at-risk")!;
    expect(atRisk.value).toBe("$0");
    expect(atRisk.sub).toMatch(/no monetary impact/);
    expect(atRisk.tone).toBe("neutral");
  });

  it("renders the populated KPI strip with non-zero numeric values across all cells", () => {
    const cells = computeKpiCells(
      [
        item({
          id: "a",
          decision: "approve",
          dollarAtRisk: 12_500,
          receivedAt: new Date(NOW - 30 * 60_000).toISOString(),
        }),
        item({
          id: "b",
          decision: null,
          dollarAtRisk: 2_500,
          receivedAt: new Date(NOW - 10 * 60_000).toISOString(),
        }),
      ],
      1,
      NOW,
    );
    expect(cells.find((c) => c.testId === "pending")?.value).toBe("1");
    expect(cells.find((c) => c.testId === "resolved")?.value).toBe("1");
    expect(cells.find((c) => c.testId === "avg-response")?.value).toBe("30m");
    expect(cells.find((c) => c.testId === "avg-response")?.ariaDescription).toBeUndefined();
    expect(cells.find((c) => c.testId === "at-risk")?.value).toMatch(/\$2,500|\$2\.5/);
    expect(cells.find((c) => c.testId === "approval-rate")?.value).toBe("100%");
  });

  it("does not carry the old fixed KPI literals in the component source", () => {
    const source = readFileSync("components/skycot/approvals/KpiStrip.tsx", "utf8");
    expect(source).not.toMatch(/["']42["']|["']8m["']|["']78%["']|0\.78|0\.14|0\.08/);
  });
});
