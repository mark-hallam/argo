/**
 * Coverage for the `/workflows/<v>/<n>` run-history swap-in. We test
 * the `run-history.tsx` components directly because the parent
 * `page.tsx` pulls in TanStack Query + Next router + edge-state modals
 * — mocking all of those would test infrastructure, not the literal-
 * swap behaviour the brief is paying for.
 *
 * Three scenarios from the brief acceptance criteria:
 *   1. renders_run_count_from_gateway
 *   2. renders_skeleton_while_loading  → renders "—" placeholders
 *   3. renders_empty_state_when_zero_runs
 */
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { WorkflowRun } from "@/lib/api/workflow-runs";
import { WorkflowRunStats, WorkflowRunsTable } from "./run-history";

function summary(overrides: Partial<{
  run_count: number;
  success_rate: number | null;
  avg_duration_ms: number;
  last_run_at: string | null;
}> = {}) {
  return {
    run_count: 82,
    success_rate: 0.94,
    avg_duration_ms: 4321,
    last_run_at: "2026-05-20T14:32:00Z",
    ...overrides,
  };
}

describe("WorkflowRunStats", () => {
  it("renders_run_count_from_gateway", () => {
    render(
      <WorkflowRunStats
        query={{ isPending: false, isError: false }}
        summary={summary({ run_count: 47, success_rate: 0.91 })}
      />,
    );
    expect(screen.getByTestId("wfd-stat-runs")).toHaveTextContent("47");
    expect(screen.getByTestId("wfd-stat-success-rate")).toHaveTextContent("91%");
  });

  it("renders_skeleton_while_loading", () => {
    render(
      <WorkflowRunStats
        query={{ isPending: true, isError: false }}
        summary={null}
      />,
    );
    // Each cell shows the em-dash placeholder while the query is
    // pending — verify all four cells are in the skeleton state and
    // that no real number sneaks through.
    expect(screen.getByTestId("wfd-stat-runs")).toHaveTextContent("—");
    expect(screen.getByTestId("wfd-stat-success-rate")).toHaveTextContent("—");
    expect(screen.getByTestId("wfd-stat-avg-duration")).toHaveTextContent("—");
    expect(screen.getByTestId("wfd-stat-last-run")).toHaveTextContent("—");
    expect(screen.queryByTestId("wfd-runs-empty")).toBeNull();
  });

  it("renders_empty_state_when_zero_runs", () => {
    render(
      <WorkflowRunStats
        query={{ isPending: false, isError: false }}
        summary={summary({
          run_count: 0,
          success_rate: null,
          last_run_at: null,
        })}
      />,
    );
    expect(screen.getByTestId("wfd-stat-runs")).toHaveTextContent("0");
    // Zero of zero is undefined — must render the dash, not "0%".
    expect(screen.getByTestId("wfd-stat-success-rate")).toHaveTextContent("—");
    expect(screen.getByTestId("wfd-stat-last-run")).toHaveTextContent("Never");
    expect(screen.getByTestId("wfd-runs-empty")).toBeInTheDocument();
  });

  it("does NOT render the empty-state nudge while loading", () => {
    render(
      <WorkflowRunStats
        query={{ isPending: true, isError: false }}
        summary={null}
      />,
    );
    expect(screen.queryByTestId("wfd-runs-empty")).toBeNull();
  });
});

describe("WorkflowRunsTable", () => {
  it("renders one row per gateway run", () => {
    const runs: WorkflowRun[] = [
      {
        id: "wfr_aaaaaaaa11",
        started_at: new Date(Date.now() - 60_000).toISOString(),
        finished_at: new Date(Date.now() - 55_000).toISOString(),
        ok: true,
        duration_ms: 5_000,
      },
      {
        id: "wfr_bbbbbbbb22",
        started_at: new Date(Date.now() - 3_600_000).toISOString(),
        finished_at: null,
        ok: false,
        duration_ms: null,
      },
    ];
    render(
      <WorkflowRunsTable isPending={false} isError={false} runs={runs} />,
    );
    expect(screen.getByTestId("wfd-runs-table")).toBeInTheDocument();
    expect(screen.getAllByRole("row")).toHaveLength(3); // header + 2 rows
    expect(screen.getByText(/wfr_aaaa/)).toBeInTheDocument();
    expect(screen.getByText("Succeeded")).toBeInTheDocument();
    expect(screen.getByText("Failed")).toBeInTheDocument();
    expect(screen.getByText("Running…")).toBeInTheDocument();
  });

  it("hides the table while the query is pending or on empty data", () => {
    const { rerender } = render(
      <WorkflowRunsTable isPending={true} isError={false} runs={[]} />,
    );
    expect(screen.queryByTestId("wfd-runs-table")).toBeNull();
    rerender(
      <WorkflowRunsTable isPending={false} isError={false} runs={[]} />,
    );
    expect(screen.queryByTestId("wfd-runs-table")).toBeNull();
    rerender(
      <WorkflowRunsTable isPending={false} isError={true} runs={[]} />,
    );
    expect(screen.queryByTestId("wfd-runs-table")).toBeNull();
  });
});
