import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { WorkflowSpec } from "@/app/(app)/workflows/[verticalId]/[name]/build/types";
import { MobileBuilderGate } from "./MobileBuilderGate";

const SAMPLE_WORKFLOW: WorkflowSpec = {
  name: "post_listing",
  vertical: "gtm-engineer",
  version: "v3",
  description: "Validate inventory, generate copy, post to Marketplace.",
  risk: "approval-required",
  steps: [
    {
      id: "s1",
      kind: "trigger",
      name: "Seller submits inventory",
      description: "Slack/email/web inbox with photos.",
    },
    {
      id: "s2",
      kind: "approval",
      name: "Seller approves",
      description: "One-tap approve via Slack.",
      approvalGate: { reason: "Public listing needs signoff.", channel: "Slack" },
    },
    {
      id: "s3",
      kind: "loop",
      name: "For each photo",
      description: "Loop over uploaded photos.",
      body: [
        {
          id: "s3a",
          kind: "skill_invoke",
          name: "OCR + tag",
          description: "Tag each photo with detected categories.",
        },
      ],
    },
    {
      id: "s4",
      kind: "decision",
      name: "Below floor?",
      description: "Branch on validation result.",
      branch: {
        trueLabel: "passes",
        falseLabel: "below floor",
        falseSteps: [
          {
            id: "s4b1",
            kind: "approval",
            name: "Manager reviews",
            description: "Floor exceptions need manager OK.",
          },
        ],
      },
    },
    {
      id: "s5",
      kind: "sub_workflow",
      name: "Notify CRM",
      description: "Hand off to the CRM-sync sub-workflow.",
      subWorkflow: {
        verticalId: "ops",
        name: "crm_sync",
        displayName: "crm sync",
        stepCount: 3,
      },
    },
  ],
};

const writeText = vi.fn(async (_text: string) => undefined);

function installClipboardSpy() {
  Object.defineProperty(globalThis.navigator, "clipboard", {
    configurable: true,
    writable: true,
    value: { writeText },
  });
}

installClipboardSpy();

beforeEach(() => {
  writeText.mockClear();
  installClipboardSpy();
  Object.defineProperty(window, "location", {
    configurable: true,
    value: {
      ...window.location,
      href: "https://app.skycot.test/workflows/gtm/post_listing/build",
    },
  });
});

describe("MobileBuilderGate", () => {
  it("renders the workflow name, vertical tag, and a step count", () => {
    render(<MobileBuilderGate workflow={SAMPLE_WORKFLOW} />);
    expect(screen.getByRole("heading", { name: /open this on desktop to edit/i })).toBeInTheDocument();
    expect(screen.getByText("post_listing", { selector: "h2" })).toBeInTheDocument();
    expect(screen.getByText(/gtm-engineer · version v3/i)).toBeInTheDocument();
    expect(screen.getByText(/^5 steps$/)).toBeInTheDocument();
  });

  it("renders each top-level step as a read-only card", () => {
    render(<MobileBuilderGate workflow={SAMPLE_WORKFLOW} />);
    for (const step of SAMPLE_WORKFLOW.steps) {
      expect(screen.getByTestId(`mobile-gate-step-${step.id}`)).toBeInTheDocument();
      expect(screen.getByText(step.name, { selector: "h3" })).toBeInTheDocument();
    }
  });

  it("renders the approval-gate badge for steps that have one", () => {
    render(<MobileBuilderGate workflow={SAMPLE_WORKFLOW} />);
    expect(screen.getByText(/gate · slack/i)).toBeInTheDocument();
  });

  it("renders loop body steps under their parent", () => {
    render(<MobileBuilderGate workflow={SAMPLE_WORKFLOW} />);
    expect(screen.getByTestId("mobile-gate-step-s3-body")).toBeInTheDocument();
    expect(screen.getByTestId("mobile-gate-step-s3a")).toBeInTheDocument();
  });

  it("renders the decision-branch false path with its steps", () => {
    render(<MobileBuilderGate workflow={SAMPLE_WORKFLOW} />);
    expect(screen.getByTestId("mobile-gate-step-s4-branch")).toBeInTheDocument();
    expect(screen.getByTestId("mobile-gate-step-s4b1")).toBeInTheDocument();
    expect(screen.getByText(/below floor path/i)).toBeInTheDocument();
  });

  it("renders the sub-workflow link badge for steps with a subWorkflow ref", () => {
    render(<MobileBuilderGate workflow={SAMPLE_WORKFLOW} />);
    expect(screen.getByTestId("mobile-gate-step-s5-subworkflow")).toBeInTheDocument();
    expect(screen.getByText(/calls crm sync · 3 steps/i)).toBeInTheDocument();
  });

  it("uses the mobile-gate class so the CSS media query can toggle visibility", () => {
    render(<MobileBuilderGate workflow={SAMPLE_WORKFLOW} />);
    expect(screen.getByTestId("mobile-builder-gate")).toHaveClass("mobile-gate");
  });

  it("copies the current URL to the clipboard and flips the button label", async () => {
    render(<MobileBuilderGate workflow={SAMPLE_WORKFLOW} />);
    const button = screen.getByTestId("mobile-gate-copy-link");
    expect(button).toHaveTextContent(/copy link/i);
    await act(async () => {
      fireEvent.click(button);
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText).toHaveBeenCalledWith(
      "https://app.skycot.test/workflows/gtm/post_listing/build",
    );
    expect(button).toHaveTextContent(/link copied/i);
  });
});
