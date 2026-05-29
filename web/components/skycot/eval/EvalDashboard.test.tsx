import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { EvalFixtureTable } from "@/components/skycot/eval/EvalFixtureTable";
import { EvalRunHistory } from "@/components/skycot/eval/EvalRunHistory";
import { RegressionBanner } from "@/components/skycot/eval/RegressionBanner";
import { mockEvalDashboard, type EvalDashboardPayload, type EvalRun } from "@/lib/api/evals";

// `mockEvalDashboard` always produces a concrete `latest` run; the
// dashboard payload type widened `latest` to `EvalRun | null` (a real
// empty when a worker has no reeval history), so narrow it back here so
// the component props below stay non-null without per-line assertions.
function payload(): EvalDashboardPayload & { latest: EvalRun } {
  return mockEvalDashboard("wkr_eval", "mock-bypass") as EvalDashboardPayload & { latest: EvalRun };
}

describe("eval dashboard components", () => {
  it("RegressionBanner summarizes blocking regressions", () => {
    render(<RegressionBanner run={payload().latest} />);
    expect(screen.getByTestId("regression-banner")).toHaveTextContent("Regression detected");
    expect(screen.getByTestId("regression-banner")).toHaveTextContent("need review");
  });

  it("EvalFixtureTable shows pass/fail status and score deltas", () => {
    render(<EvalFixtureTable fixtures={payload().latest.fixtures} />);
    const row = screen.getByTestId("eval-fixture-reply-late-stage-candidate");
    expect(within(row).getByTestId("eval-fixture-status-reply-late-stage-candidate")).toHaveTextContent(
      "failed",
    );
    expect(row).toHaveTextContent("-60 pts");
  });

  it("EvalRunHistory calls onSelect for the selected run", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const runs = payload().history;
    render(<EvalRunHistory runs={runs} selectedRunId={runs[0].id} onSelect={onSelect} />);
    await user.click(screen.getByTestId("eval-run-eval_run_baseline"));
    expect(onSelect).toHaveBeenCalledWith("eval_run_baseline");
  });
});
