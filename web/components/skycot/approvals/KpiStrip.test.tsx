import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { KpiStrip, type KpiCell } from "./KpiStrip";

const SAMPLE_CELLS: KpiCell[] = [
  { label: "Pending", value: "3", sub: "active human decisions", testId: "pending" },
  { label: "$ at risk", value: "$1,200", sub: "across 1 pending", tone: "warn", testId: "at-risk" },
  { label: "Median time", value: "12m", sub: "waiting for approval", testId: "median-time" },
  { label: "Approval rate", value: "—", sub: "no resolved approvals", testId: "approval-rate" },
  { label: "Active reviewers", value: "—", sub: "reviewer roster pending", testId: "active-reviewers" },
];

describe("KpiStrip chrome", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "matchMedia",
      vi.fn().mockReturnValue({ matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() }),
    );
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders one cell per entry with the provided label, value and sub", () => {
    render(<KpiStrip cells={SAMPLE_CELLS} />);
    for (const cell of SAMPLE_CELLS) {
      const el = screen.getByTestId(`kpi-${cell.testId}`);
      expect(el).toHaveTextContent(cell.label);
      expect(el).toHaveTextContent(cell.value);
      if (cell.sub) expect(el).toHaveTextContent(cell.sub);
    }
  });

  it("annotates the tone on each cell so styling hooks can target it", () => {
    render(<KpiStrip cells={SAMPLE_CELLS} />);
    expect(screen.getByTestId("kpi-at-risk")).toHaveAttribute("data-tone", "warn");
    expect(screen.getByTestId("kpi-pending")).toHaveAttribute("data-tone", "neutral");
  });

  it("omits the 7-day rate ring when no cell carries derived rate data", () => {
    render(<KpiStrip cells={SAMPLE_CELLS} />);
    expect(screen.queryByTestId("kpi-rate-ring")).not.toBeInTheDocument();
    expect(screen.queryByTestId("approval-rate-ring")).not.toBeInTheDocument();
  });

  it("renders the 7-day rate ring when an approval-rate cell carries rate slices", () => {
    const cellsWithRate: KpiCell[] = [
      ...SAMPLE_CELLS.slice(0, 3),
      {
        label: "Approval rate",
        value: "67%",
        sub: "3 resolved approvals",
        tone: "good",
        testId: "approval-rate",
        rate: {
          headline: "67%",
          slices: { approved: 0.67, refused: 0.33, expired: 0 },
        },
      },
      SAMPLE_CELLS[4],
    ];
    render(<KpiStrip cells={cellsWithRate} />);
    expect(screen.getByTestId("kpi-rate-ring")).toHaveTextContent(/7-day approval rate/);
    expect(screen.getByTestId("approval-rate-ring")).toHaveAttribute("data-approved", "67");
  });
});
