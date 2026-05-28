import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SAMPLE_WORKFLOW_SUMMARIES } from "@/lib/sample-workflows";

const hooksApi = vi.hoisted(() => ({
  useWorkflows: vi.fn(),
}));

vi.mock("@/lib/hooks/use-workflows", () => hooksApi);

vi.mock("@/components/skycot/WorkflowProposeModal", () => ({
  WorkflowProposeModal: () => <div data-testid="propose-modal-mock" />,
}));

vi.mock("@/components/skycot/workflow/SubstrateChip", () => ({
  SubstrateChip: () => <span data-testid="substrate-chip-mock" />,
}));

vi.mock("@/components/ui/Tooltip", () => ({
  HoverHint: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import WorkflowsPage from "../page";

describe("/workflows editorial intro band", () => {
  beforeEach(() => {
    hooksApi.useWorkflows.mockReturnValue({
      data: { workflows: SAMPLE_WORKFLOW_SUMMARIES, source: "mock-bypass" },
      isLoading: false,
      isError: false,
    });
  });

  it("preserves the brand-loud H1 'how your teammates actually think'", () => {
    render(<WorkflowsPage />);
    const h1 = screen.getByRole("heading", { level: 1 });
    expect(h1).toHaveTextContent(/how your teammates/i);
    expect(h1).toHaveTextContent(/actually/i);
    expect(h1).toHaveTextContent(/think/i);
  });

  it("renders the editorial intro band between hero and search row", () => {
    render(<WorkflowsPage />);
    const band = screen.getByTestId("workflows-intro-band");
    expect(band).toBeInTheDocument();
    expect(band).toHaveTextContent(/sequence of decisions/i);
    expect(band).toHaveTextContent(/whole decision chain/i);
  });

  it("links to the workflow library from the intro band", () => {
    render(<WorkflowsPage />);
    const link = screen.getByTestId("workflows-intro-deep-link");
    expect(link).toHaveAttribute("href", "/workflows/templates");
    expect(link).toHaveTextContent(/open the workflow library/i);
  });
});
