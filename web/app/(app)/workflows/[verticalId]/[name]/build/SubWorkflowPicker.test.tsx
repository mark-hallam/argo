import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { FetchAllWorkflowsResult } from "@/lib/api/workflows";
import { useWorkflows } from "@/lib/hooks/use-workflows";
import { SubWorkflowPicker } from "./SubWorkflowPicker";

vi.mock("@/lib/hooks/use-workflows", () => ({
  useWorkflows: vi.fn(),
}));

const mockUseWorkflows = vi.mocked(useWorkflows);

function renderPicker() {
  const onPick = vi.fn();
  const onClose = vi.fn();
  render(
    <SubWorkflowPicker
      currentVerticalId="sales"
      currentName="current_workflow"
      onPick={onPick}
      onClose={onClose}
    />,
  );
  return { onPick, onClose };
}

function result(workflows: FetchAllWorkflowsResult["workflows"]) {
  return {
    data: { workflows, source: "gateway" },
    isLoading: false,
  } as ReturnType<typeof useWorkflows>;
}

describe("SubWorkflowPicker", () => {
  it("uses gateway workflows before local seed workflows", () => {
    mockUseWorkflows.mockReturnValue(
      result([
        {
          name: "customer_research_followup",
          vertical: "sales",
          description: "Gateway workflow from the live workflow API.",
          steps: 2,
          versions: 1,
          lastEdited: "now",
          lastEditor: "Peta",
          risk: "approval-required",
        },
      ]),
    );
    const { onPick } = renderPicker();

    expect(
      screen.getByTestId(
        "sub-workflow-picker-option-sales-customer_research_followup",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId("sub-workflow-picker-option-gtm-engineer-post_listing"),
    ).toBeNull();

    fireEvent.click(
      screen.getByTestId(
        "sub-workflow-picker-option-sales-customer_research_followup",
      ),
    );
    expect(onPick).toHaveBeenCalledWith({
      verticalId: "sales",
      name: "customer_research_followup",
      displayName: "customer research followup",
      stepCount: 2,
    });
  });

  it("falls back to seed workflows only when the gateway list is empty", () => {
    mockUseWorkflows.mockReturnValue(result([]));
    renderPicker();

    expect(
      screen.getByTestId("sub-workflow-picker-option-gtm-engineer-post_listing"),
    ).toBeInTheDocument();
  });

  it("shows a loading state without rendering seed options while the query is pending", () => {
    mockUseWorkflows.mockReturnValue({
      data: undefined,
      isLoading: true,
    } as ReturnType<typeof useWorkflows>);
    renderPicker();

    expect(screen.getByTestId("sub-workflow-picker-loading")).toBeInTheDocument();
    expect(
      screen.queryByTestId("sub-workflow-picker-option-gtm-engineer-post_listing"),
    ).toBeNull();
  });
});
