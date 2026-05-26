import { ReactFlowProvider } from "@xyflow/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ComponentProps } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { SubWorkflowNode } from "@/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowNode";
import { SubWorkflowPicker } from "@/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker";
import { newStepNode } from "@/app/(app)/workflows/[verticalId]/[name]/build/builder-handlers";
import { stepsToFlow } from "@/app/(app)/workflows/[verticalId]/[name]/build/flow-layout";
import {
  PALETTE_GROUPS,
  STEP_KIND_META,
  type SkyNodeData,
  type SubWorkflowRef,
  type WorkflowStep,
} from "@/app/(app)/workflows/[verticalId]/[name]/build/types";
import { SAMPLE_WORKFLOW_SUMMARIES } from "@/lib/sample-workflows";

const sampleRef: SubWorkflowRef = {
  verticalId: "gtm-engineer",
  name: "post_listing",
  displayName: "post listing",
  stepCount: 6,
};

function renderNode(step: WorkflowStep) {
  return render(
    <ReactFlowProvider>
      <SubWorkflowNode
        id={step.id}
        type="subWorkflow"
        data={{ step } as SkyNodeData}
        selected={false}
        zIndex={0}
        isConnectable
        positionAbsoluteX={0}
        positionAbsoluteY={0}
        dragging={false}
        draggable
        selectable
        deletable
      />
    </ReactFlowProvider>,
  );
}

function renderPicker(
  props: Omit<ComponentProps<typeof SubWorkflowPicker>, "onPick" | "onClose"> & {
    onPick?: (ref: SubWorkflowRef) => void;
    onClose?: () => void;
  },
) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        staleTime: Infinity,
      },
    },
  });
  queryClient.setQueryData(["workflows"], {
    workflows: SAMPLE_WORKFLOW_SUMMARIES,
    source: "mock-offline",
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <SubWorkflowPicker
        {...props}
        onPick={props.onPick ?? (() => {})}
        onClose={props.onClose ?? (() => {})}
      />
    </QueryClientProvider>,
  );
}

describe("SubWorkflowNode", () => {
  it("renders the sub-workflow display name, vertical tag, and step-count badge", () => {
    const step: WorkflowStep = {
      id: "s2",
      kind: "sub_workflow",
      name: "Call post_listing",
      description: "Invokes gtm-engineer/post_listing.",
      subWorkflow: sampleRef,
    };
    renderNode(step);

    expect(screen.getByTestId("sub-workflow-node")).toBeInTheDocument();
    expect(screen.getByText("post listing")).toBeInTheDocument();
    expect(screen.getByTestId("sub-workflow-vertical").textContent).toBe("gtm-engineer");
    expect(screen.getByTestId("sub-workflow-step-count").textContent).toBe("6 steps");
  });

  it("renders the Expand link with target=_blank pointing at the sub-workflow's builder", () => {
    const step: WorkflowStep = {
      id: "s2",
      kind: "sub_workflow",
      name: "Call post_listing",
      description: "Invokes gtm-engineer/post_listing.",
      subWorkflow: sampleRef,
    };
    renderNode(step);

    const expand = screen.getByTestId("sub-workflow-expand") as HTMLAnchorElement;
    expect(expand.getAttribute("href")).toBe("/workflows/gtm-engineer/post_listing/build");
    expect(expand.getAttribute("target")).toBe("_blank");
    expect(expand.getAttribute("rel")).toBe("noopener noreferrer");
  });

  it("renders the empty-state hint when no sub-workflow has been picked yet", () => {
    const step: WorkflowStep = {
      id: "s2",
      kind: "sub_workflow",
      name: "Sub-workflow",
      description: "Invoke another workflow as a single step.",
    };
    renderNode(step);

    expect(screen.getByTestId("sub-workflow-empty")).toBeInTheDocument();
    expect(screen.queryByTestId("sub-workflow-expand")).not.toBeInTheDocument();
  });

  it("singularises the step-count badge for 1-step sub-workflows", () => {
    const step: WorkflowStep = {
      id: "s2",
      kind: "sub_workflow",
      name: "Call thin_flow",
      description: "Invokes a 1-step sub-workflow.",
      subWorkflow: { ...sampleRef, stepCount: 1 },
    };
    renderNode(step);

    expect(screen.getByTestId("sub-workflow-step-count").textContent).toBe("1 step");
  });
});

describe("SubWorkflowPicker", () => {
  it("excludes the current workflow from the candidate list", () => {
    renderPicker({
      currentVerticalId: "gtm-engineer",
      currentName: "post_listing",
    });

    expect(screen.getByTestId("sub-workflow-picker")).toBeInTheDocument();
    expect(
      screen.queryByTestId("sub-workflow-picker-option-gtm-engineer-post_listing"),
    ).not.toBeInTheDocument();
    // The other catalog fallback workflows are present.
    expect(
      screen.getByTestId("sub-workflow-picker-option-sales-qualify_lead"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("sub-workflow-picker-option-support-approve_refund"),
    ).toBeInTheDocument();
  });

  it("filters candidates by the search input", async () => {
    const user = userEvent.setup();
    renderPicker({
      currentVerticalId: "other",
      currentName: "none",
    });

    const search = screen.getByTestId("sub-workflow-picker-search");
    await user.type(search, "approve_refund");

    expect(
      screen.getByTestId("sub-workflow-picker-option-support-approve_refund"),
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId("sub-workflow-picker-option-sales-qualify_lead"),
    ).not.toBeInTheDocument();
  });

  it("calls onPick with the chosen workflow ref when a candidate is clicked", async () => {
    const user = userEvent.setup();
    const onPick = vi.fn();
    renderPicker({
      currentVerticalId: "other",
      currentName: "none",
      onPick,
    });

    await user.click(
      screen.getByTestId("sub-workflow-picker-option-sales-qualify_lead"),
    );

    expect(onPick).toHaveBeenCalledTimes(1);
    const ref = onPick.mock.calls[0]?.[0] as SubWorkflowRef;
    expect(ref.verticalId).toBe("sales");
    expect(ref.name).toBe("qualify_lead");
    expect(ref.stepCount).toBeGreaterThan(0);
  });

  it("renders an empty-state when the search matches nothing", async () => {
    const user = userEvent.setup();
    renderPicker({
      currentVerticalId: "other",
      currentName: "none",
    });

    const search = screen.getByTestId("sub-workflow-picker-search");
    await user.type(search, "zzzzz-no-match");

    expect(screen.getByTestId("sub-workflow-picker-empty")).toBeInTheDocument();
  });
});

describe("sub-workflow plumbing", () => {
  it("registers sub_workflow under the Composition group in the palette", () => {
    const group = PALETTE_GROUPS.find((g) => g.label === "Composition");
    expect(group).toBeDefined();
    expect(group?.kinds).toContain("sub_workflow");
  });

  it("includes a sub_workflow entry in STEP_KIND_META", () => {
    expect(STEP_KIND_META.sub_workflow).toBeDefined();
    expect(STEP_KIND_META.sub_workflow.label).toBe("Sub-workflow");
  });

  it("stepsToFlow tags sub_workflow steps with the subWorkflow node type", () => {
    const steps: WorkflowStep[] = [
      { id: "s1", kind: "trigger", name: "Start", description: "Entry." },
      {
        id: "s2",
        kind: "sub_workflow",
        name: "Call",
        description: "Invokes.",
        subWorkflow: sampleRef,
      },
    ];
    const { nodes } = stepsToFlow(steps);

    expect(nodes[0]?.type).toBe("skycot");
    expect(nodes[1]?.type).toBe("subWorkflow");
  });

  it("newStepNode builds a sub_workflow node with the given ref + tags it subWorkflow", () => {
    const node = newStepNode("sub_workflow", [], "abc12345", sampleRef);
    expect(node.type).toBe("subWorkflow");
    const step = (node.data as SkyNodeData).step;
    expect(step.kind).toBe("sub_workflow");
    expect(step.subWorkflow).toEqual(sampleRef);
    expect(step.name).toBe("Call post listing");
  });
});
