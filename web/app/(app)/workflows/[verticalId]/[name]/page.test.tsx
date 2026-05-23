/**
 * Coverage for the `/workflows/<v>/<n>` run-history surface.
 *
 * Tests the building-block components (`run-history.tsx`,
 * `runs-filter-bar.tsx`) directly because the parent `page.tsx` pulls
 * in TanStack Query + Next router — mocking all of those would test
 * infrastructure, not the behaviour the brief is paying for.
 *
 * Coverage spans:
 *   - Stat grid (renders gateway numbers, skeleton, empty state)
 *   - Recent-runs table (renders rows, hides on empty, shows
 *     filtered-empty state with Clear link, shows skeleton during
 *     pending)
 *   - Filter bar (status chip toggle, page-size dropdown, prev/next,
 *     showing X-Y of Z indicator, clear-all)
 */
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { WorkflowRun } from "@/lib/api/workflow-runs";
import { WorkflowRunStats, WorkflowRunsTable } from "./run-history";
import {
  WorkflowRunsFilterBar,
  DEFAULT_FILTER_STATE,
  type RunsFilterState,
} from "./runs-filter-bar";

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

  it("renders skeleton rows while pending", () => {
    render(<WorkflowRunsTable isPending={true} isError={false} runs={[]} />);
    expect(screen.getByTestId("wfd-runs-skeleton")).toBeInTheDocument();
    expect(screen.queryByTestId("wfd-runs-table")).toBeNull();
  });

  it("hides the table on empty + no active filters (legacy behaviour)", () => {
    render(<WorkflowRunsTable isPending={false} isError={false} runs={[]} />);
    expect(screen.queryByTestId("wfd-runs-table")).toBeNull();
    expect(screen.queryByTestId("wfd-runs-filtered-empty")).toBeNull();
  });

  it("renders filtered-empty state with Clear link", async () => {
    const onClear = vi.fn();
    const user = userEvent.setup();
    render(
      <WorkflowRunsTable
        isPending={false}
        isError={false}
        runs={[]}
        filtersActive={true}
        onClearFilters={onClear}
      />,
    );
    expect(screen.getByTestId("wfd-runs-filtered-empty")).toHaveTextContent(
      /no runs match your filters/i,
    );
    await user.click(screen.getByTestId("wfd-runs-clear-link"));
    expect(onClear).toHaveBeenCalledTimes(1);
  });

  it("hides everything on error", () => {
    render(
      <WorkflowRunsTable
        isPending={false}
        isError={true}
        runs={[]}
        filtersActive={true}
      />,
    );
    expect(screen.queryByTestId("wfd-runs-table")).toBeNull();
    expect(screen.queryByTestId("wfd-runs-skeleton")).toBeNull();
    expect(screen.queryByTestId("wfd-runs-filtered-empty")).toBeNull();
  });
});

describe("WorkflowRunsFilterBar", () => {
  function setup(overrides: Partial<RunsFilterState> = {}) {
    const onChange = vi.fn();
    const state: RunsFilterState = { ...DEFAULT_FILTER_STATE, ...overrides };
    const utils = render(
      <WorkflowRunsFilterBar
        state={state}
        onChange={onChange}
        totalFiltered={120}
        visibleCount={25}
      />,
    );
    return { onChange, ...utils };
  }

  it("renders the showing X-Y of Z indicator with comma-separated totals", () => {
    const { rerender } = setup();
    expect(screen.getByTestId("wfd-pagination-indicator")).toHaveTextContent(
      "Showing 1-25 of 120",
    );
    rerender(
      <WorkflowRunsFilterBar
        state={{ ...DEFAULT_FILTER_STATE, page: 2 }}
        onChange={() => undefined}
        totalFiltered={1247}
        visibleCount={25}
      />,
    );
    expect(screen.getByTestId("wfd-pagination-indicator")).toHaveTextContent(
      "Showing 26-50 of 1,247",
    );
  });

  it("toggles a status chip and resets page", async () => {
    const user = userEvent.setup();
    const { onChange } = setup({ page: 3 });
    await user.click(screen.getByTestId("wfd-status-failed"));
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ statuses: ["failed"], page: 1 }),
    );
  });

  it("supports multi-select status (toggle removes)", async () => {
    const user = userEvent.setup();
    const { onChange } = setup({ statuses: ["failed", "running"] });
    await user.click(screen.getByTestId("wfd-status-failed"));
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ statuses: ["running"], page: 1 }),
    );
  });

  it("changes page size and resets page", async () => {
    const user = userEvent.setup();
    const { onChange } = setup({ page: 4 });
    await user.selectOptions(screen.getByTestId("wfd-page-size"), "50");
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ pageSize: 50, page: 1 }),
    );
  });

  it("Prev disabled on page 1, Next disabled on last page", () => {
    setup({ page: 1, pageSize: 25 });
    expect(screen.getByTestId("wfd-prev-page")).toBeDisabled();
    expect(screen.getByTestId("wfd-next-page")).toBeEnabled();
  });

  it("Next disabled when page covers all rows", () => {
    render(
      <WorkflowRunsFilterBar
        state={{ ...DEFAULT_FILTER_STATE, page: 5, pageSize: 25 }}
        onChange={() => undefined}
        totalFiltered={120}
        visibleCount={20}
      />,
    );
    expect(screen.getByTestId("wfd-next-page")).toBeDisabled();
  });

  it("range preset switches to custom and surfaces date inputs", async () => {
    const user = userEvent.setup();
    const { onChange } = setup();
    await user.click(screen.getByTestId("wfd-range-custom"));
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ range: "custom", page: 1 }),
    );
  });

  it("renders 0 of 0 when totalFiltered is zero", () => {
    render(
      <WorkflowRunsFilterBar
        state={{ ...DEFAULT_FILTER_STATE, statuses: ["queued"] }}
        onChange={() => undefined}
        totalFiltered={0}
        visibleCount={0}
      />,
    );
    expect(screen.getByTestId("wfd-pagination-indicator")).toHaveTextContent(
      /0 of 0/i,
    );
  });

  it("Clear button appears only when filters are non-default", async () => {
    const user = userEvent.setup();
    const { onChange, rerender } = setup();
    expect(screen.queryByTestId("wfd-clear-filters")).toBeNull();
    rerender(
      <WorkflowRunsFilterBar
        state={{ ...DEFAULT_FILTER_STATE, statuses: ["success"] }}
        onChange={onChange}
        totalFiltered={5}
        visibleCount={5}
      />,
    );
    await user.click(screen.getByTestId("wfd-clear-filters"));
    expect(onChange).toHaveBeenCalledWith(DEFAULT_FILTER_STATE);
  });
});
