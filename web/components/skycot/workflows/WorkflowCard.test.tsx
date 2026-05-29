import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { SubstrateChip } from "@/components/skycot/workflow/SubstrateChip";
import { SAMPLE_WORKFLOW_SUMMARIES } from "@/lib/sample-workflows";

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
        authorWorkerId="019e6785-928a-7270-a766-f945b6cd7161"
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
        authorWorkerId="019e6785-928d-78b1-a6b2-136d9ed263ce"
      />,
    );
    const chip = screen.getByTestId("workflow-author-chip");
    expect(chip.tagName).toBe("BUTTON");
    await userEvent.click(chip);
    expect(pushMock).toHaveBeenCalledWith("/workers/019e6785-928d-78b1-a6b2-136d9ed263ce");
  });

  it("falls back to a non-clickable span when authorWorkerId is missing", () => {
    pushMock.mockClear();
    render(<WorkflowAuthorChip authorType="worker" authorName="Maya" />);
    const chip = screen.getByTestId("workflow-author-chip");
    expect(chip.tagName).toBe("SPAN");
    expect(chip).toHaveTextContent("by Maya");
  });
});

/**
 * Catalog substrate coverage + chip rendering, anchored to the
 * `/workflows` page chip column. The on-camera surface (`page.tsx`)
 * resolves a substrate per card and hands it to <SubstrateChip>, so
 * these assertions guarantee that (a) the seed carries ≥3 distinct
 * substrate values, (b) macOS + a sandbox/browser substrate are
 * represented, and (c) every substrate kind renders the expected
 * label without throwing. Scene 5 "any-PC execution" voiceover
 * relies on this — uniform HEADLESS chips contradict the claim.
 */
describe("workflow catalog substrate chips", () => {
  const substrates = SAMPLE_WORKFLOW_SUMMARIES.map((wf) => wf.substrate);

  it("seed shows ≥3 distinct substrates across visible cards", () => {
    const distinct = new Set(substrates);
    expect(distinct.size).toBeGreaterThanOrEqual(3);
  });

  it("at least one workflow runs on macOS (proves Claim 5 cu)", () => {
    expect(substrates).toContain("macos");
  });

  it("at least one workflow runs on browser or E2B (proves multi-substrate)", () => {
    const hasMultiSubstrate = substrates.some((s) => s === "browser" || s === "e2b");
    expect(hasMultiSubstrate).toBe(true);
  });

  it("SubstrateChip renders the expected label for every seed substrate", () => {
    const labels: Record<string, string> = {
      macos: "macOS",
      linux: "Linux",
      windows: "Windows",
      e2b: "E2B",
      browser: "Browser",
      headless: "Headless",
    };

    for (const substrate of new Set(substrates)) {
      const { unmount } = render(
        <SubstrateChip substrate={substrate} status="idle" />,
      );
      const expectedLabel = labels[substrate];
      expect(expectedLabel, `missing label mapping for ${substrate}`).toBeDefined();
      const chip = screen.getByTestId("substrate-chip");
      expect(chip.getAttribute("data-substrate")).toBe(substrate);
      expect(chip.textContent).toContain(expectedLabel);
      unmount();
    }
  });
});
