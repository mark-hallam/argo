import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import {
  OrchestrationEmptyState,
  WorkflowBuilderEmptyState,
  WorkflowBuilderErrorState,
  WorkflowViewerErrorState,
} from "./WorkflowEdgeStates";

describe("WorkflowBuilderErrorState", () => {
  it("renders the alert headline, gateway copy, and a back-to-workflows link", () => {
    render(<WorkflowBuilderErrorState />);
    expect(screen.getByTestId("workflow-builder-error-state")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /couldn't load the workflow/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /back to workflows/i })).toHaveAttribute("href", "/workflows");
  });

  it("only shows the retry button when onRetry is provided and invokes it on click", async () => {
    const onRetry = vi.fn();
    const user = userEvent.setup();
    const { rerender } = render(<WorkflowBuilderErrorState onRetry={onRetry} />);
    await user.click(screen.getByTestId("workflow-builder-error-retry"));
    expect(onRetry).toHaveBeenCalledTimes(1);

    rerender(<WorkflowBuilderErrorState />);
    expect(screen.queryByTestId("workflow-builder-error-retry")).toBeNull();
  });
});

describe("WorkflowBuilderEmptyState", () => {
  it("renders the empty headline + CTA that wires the onAddStep callback", async () => {
    const onAddStep = vi.fn();
    const user = userEvent.setup();
    render(<WorkflowBuilderEmptyState onAddStep={onAddStep} />);
    expect(
      screen.getByRole("heading", { name: /this workflow has no steps yet/i }),
    ).toBeInTheDocument();
    await user.click(screen.getByTestId("workflow-builder-empty-add-step"));
    expect(onAddStep).toHaveBeenCalledTimes(1);
  });
});

describe("WorkflowViewerErrorState", () => {
  it("links back to the workflows catalog and surfaces a retry when callback is supplied", async () => {
    const onRetry = vi.fn();
    const user = userEvent.setup();
    render(<WorkflowViewerErrorState onRetry={onRetry} />);
    expect(screen.getByRole("link", { name: /back to workflows/i })).toHaveAttribute("href", "/workflows");
    await user.click(screen.getByTestId("workflow-viewer-error-retry"));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });
});

describe("OrchestrationEmptyState", () => {
  it("invites brand-new tenants to hire their first teammate", () => {
    render(<OrchestrationEmptyState />);
    expect(screen.getByTestId("orchestration-empty-state")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /no orchestration yet/i })).toBeInTheDocument();
    expect(screen.getByTestId("orchestration-empty-hire-cta")).toHaveAttribute("href", "/hire");
  });
});
