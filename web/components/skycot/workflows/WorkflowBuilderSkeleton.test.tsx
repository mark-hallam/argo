import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { WorkflowBuilderSkeleton } from "./WorkflowBuilderSkeleton";

describe("WorkflowBuilderSkeleton", () => {
  it("renders the top bar + prompt bar + 3 node placeholders + side panel", () => {
    render(<WorkflowBuilderSkeleton />);
    expect(screen.getByTestId("workflow-builder-skeleton")).toBeInTheDocument();
    expect(screen.getByTestId("workflow-builder-skeleton")).toHaveAttribute("aria-busy", "true");
    for (let i = 0; i < 3; i += 1) {
      expect(screen.getByTestId(`workflow-builder-skeleton-node-${i}`)).toBeInTheDocument();
    }
  });

  it("uses an accessible label so assistive tech announces the loading state", () => {
    render(<WorkflowBuilderSkeleton />);
    expect(screen.getByLabelText(/loading workflow builder/i)).toBeInTheDocument();
  });
});
