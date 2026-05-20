import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { WorkflowViewerSkeleton } from "./WorkflowViewerSkeleton";

describe("WorkflowViewerSkeleton", () => {
  it("renders the default 5 timeline step placeholders", () => {
    render(<WorkflowViewerSkeleton />);
    expect(screen.getByTestId("workflow-viewer-skeleton")).toBeInTheDocument();
    expect(screen.getByTestId("workflow-viewer-skeleton")).toHaveAttribute("aria-busy", "true");
    for (let i = 0; i < 5; i += 1) {
      expect(screen.getByTestId(`workflow-viewer-skeleton-step-${i}`)).toBeInTheDocument();
    }
  });

  it("honors the steps prop for variable-length workflows", () => {
    render(<WorkflowViewerSkeleton steps={3} />);
    expect(screen.getByTestId("workflow-viewer-skeleton-step-2")).toBeInTheDocument();
    expect(screen.queryByTestId("workflow-viewer-skeleton-step-3")).toBeNull();
  });
});
