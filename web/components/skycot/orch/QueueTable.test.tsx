import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { QueueTable } from "./QueueTable";
import { SlaCard } from "./SlaCard";
import { SAMPLE_ORCHESTRATION_RUNS, summarizeRuns } from "@/lib/api/orchestration";

describe("orchestration queue components", () => {
  it("renders queue rows with SLA and retry state", () => {
    render(
      <QueueTable
        runs={SAMPLE_ORCHESTRATION_RUNS.slice(0, 3)}
        onCancel={vi.fn()}
        onResetRetry={vi.fn()}
        onForceEscalate={vi.fn()}
      />,
    );
    expect(screen.getAllByTestId("orchestration-row")).toHaveLength(3);
    expect(screen.getByText("run-001")).toBeInTheDocument();
    expect(screen.getAllByTitle(/attempts/).length).toBeGreaterThan(0);
  });

  it("fires the three operator actions with the selected run id", async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    const onResetRetry = vi.fn();
    const onForceEscalate = vi.fn();
    render(
      <QueueTable
        runs={SAMPLE_ORCHESTRATION_RUNS.slice(0, 1)}
        onCancel={onCancel}
        onResetRetry={onResetRetry}
        onForceEscalate={onForceEscalate}
      />,
    );
    await user.click(screen.getByLabelText("Cancel run"));
    await user.click(screen.getByLabelText("Reset retry"));
    await user.click(screen.getByLabelText("Force escalate"));
    expect(onCancel).toHaveBeenCalledWith("run-001");
    expect(onResetRetry).toHaveBeenCalledWith("run-001");
    expect(onForceEscalate).toHaveBeenCalledWith("run-001");
  });

  it("renders SLA summary cards", () => {
    render(<SlaCard summary={summarizeRuns(SAMPLE_ORCHESTRATION_RUNS)} />);
    expect(screen.getByTestId("orchestration-sla-cards")).toBeInTheDocument();
    expect(screen.getByText("Breached")).toBeInTheDocument();
  });
});
