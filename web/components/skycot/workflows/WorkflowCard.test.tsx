import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

const pushMock = vi.hoisted(() => vi.fn());

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: pushMock,
    replace: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
}));

import { WorkflowAuthorChip } from "./WorkflowCard";

describe("WorkflowAuthorChip", () => {
  it("renders the Bot icon and worker name for worker-authored rows", () => {
    render(
      <WorkflowAuthorChip
        authorType="worker"
        authorName="Mariah"
        authorWorkerId="018f67d0-0000-7000-8000-000000000001"
      />,
    );
    const chip = screen.getByTestId("workflow-author-chip");
    expect(chip.dataset.authorType).toBe("worker");
    expect(chip).toHaveTextContent("by Mariah");
    // Bot icon comes from lucide-react and renders as an inline SVG.
    expect(chip.querySelector("svg")).not.toBeNull();
  });

  it("renders the User icon for human-authored rows", () => {
    render(<WorkflowAuthorChip authorType="human" authorName="Mark" />);
    const chip = screen.getByTestId("workflow-author-chip");
    expect(chip.dataset.authorType).toBe("human");
    expect(chip).toHaveTextContent("by Mark");
    expect(chip.tagName).toBe("SPAN");
  });

  it("renders the Layers icon for template-pack-authored rows", () => {
    render(<WorkflowAuthorChip authorType="template_pack" authorName="SKYCOT" />);
    const chip = screen.getByTestId("workflow-author-chip");
    expect(chip.dataset.authorType).toBe("template_pack");
    expect(chip).toHaveTextContent("by SKYCOT");
    expect(chip.tagName).toBe("SPAN");
  });

  it("navigates to /workers/<id> when a worker chip is clicked", async () => {
    pushMock.mockClear();
    render(
      <WorkflowAuthorChip
        authorType="worker"
        authorName="Priya"
        authorWorkerId="018f67d0-0000-7000-8000-000000000002"
      />,
    );
    const chip = screen.getByTestId("workflow-author-chip");
    expect(chip.tagName).toBe("BUTTON");
    await userEvent.click(chip);
    expect(pushMock).toHaveBeenCalledWith("/workers/018f67d0-0000-7000-8000-000000000002");
  });

  it("falls back to a non-clickable span when authorWorkerId is missing", () => {
    pushMock.mockClear();
    render(<WorkflowAuthorChip authorType="worker" authorName="Maya" />);
    const chip = screen.getByTestId("workflow-author-chip");
    expect(chip.tagName).toBe("SPAN");
    expect(chip).toHaveTextContent("by Maya");
  });
});
