import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import {
  AnnotationsPanel,
  confidenceTone,
  formatPercent,
  policyLabel,
} from "./AnnotationsPanel";
import { EDGE_COND_STROKE } from "./EdgeRenderer";
import { KIND_VISUALS, NODE_KIND_TAGS } from "./NodeRenderer";
import { SkillCanvas, computeLayout } from "./SkillCanvas";
import type { Node as SkillNode, SkillGraph } from "@/lib/types/skill-ir";

function makeNode<T extends SkillNode["kind"]>(
  id: string,
  label: string,
  kind: T,
): SkillNode {
  return { id, label, kind, annotations: {} };
}

const NINE_KIND_GRAPH: SkillGraph = {
  entry: "n_action",
  nodes: [
    makeNode("n_action", "Send email", { type: "action", instruction: "send" }),
    makeNode("n_condition", "Score check", {
      type: "condition",
      expression: "score > 0.5",
    }),
    makeNode("n_branch", "Outcome", {
      type: "branch",
      arms: [{ label: "yes", pattern: "yes" }, { label: "no" }],
    }),
    makeNode("n_await", "Await CFO", {
      type: "await-approval",
      approverRole: "cfo",
      timeoutSeconds: 3600,
    }),
    makeNode("n_fetch", "Fetch", { type: "fetch", target: "ats:apps", method: "GET" }),
    makeNode("n_transform", "Summarize", {
      type: "transform",
      template: "Tier: {{t}}",
    }),
    makeNode("n_subskill", "Compose offer", {
      type: "sub-skill-call",
      target: { skillId: "skill_offer", version: "v1.0.0" },
    }),
    makeNode("n_loop", "For each panelist", {
      type: "loop",
      iterVar: "panelist",
      body: ["n_action"],
    }),
    makeNode("n_try", "Send with retry", {
      type: "try-catch",
      tryBody: ["n_action"],
      catchBody: ["n_action"],
    }),
  ],
  edges: [
    { id: "e1", from: "n_action", to: "n_condition", cond: { kind: "always" } },
    { id: "e2", from: "n_condition", to: "n_branch", cond: { kind: "on-success" } },
    { id: "e3", from: "n_branch", to: "n_await", cond: { kind: "on-pattern", pattern: "yes" } },
    { id: "e4", from: "n_await", to: "n_fetch", cond: { kind: "on-failure" } },
    { id: "e5", from: "n_fetch", to: "n_transform", cond: { kind: "always" } },
    { id: "e6", from: "n_transform", to: "n_subskill", cond: { kind: "always" } },
    { id: "e7", from: "n_subskill", to: "n_loop", cond: { kind: "always" } },
    { id: "e8", from: "n_loop", to: "n_try", cond: { kind: "always" } },
  ],
  variables: [],
  metadata: {},
};

const NODE_WITH_ANNOTATIONS: SkillNode = {
  id: "n_action",
  label: "Send email",
  kind: { type: "action", instruction: "send" },
  annotations: {
    confidence: 0.91,
    sampleCount: 142,
    successRate: 0.88,
    lastUsedAt: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
    policy: "auto",
    toolBinding: { mcpServer: "gmail", toolName: "messages.send" },
  },
};

describe("SkillCanvas (DOM fallback)", () => {
  it("renders a node element for each of the 9 IR NodeKinds with a kind data-attribute", () => {
    render(<SkillCanvas graph={NINE_KIND_GRAPH} skipFlowRender />);

    const fallback = screen.getByTestId("skill-canvas-fallback");
    expect(fallback).toBeInTheDocument();
    expect(screen.getByTestId("skill-canvas").getAttribute("data-node-count")).toBe("9");

    const renderedKinds = new Set<string>();
    for (const node of NINE_KIND_GRAPH.nodes) {
      const el = screen.getByTestId(`canvas-node-${node.id}`);
      expect(el).toBeInTheDocument();
      const kind = el.getAttribute("data-kind");
      expect(kind).toBe(node.kind.type);
      if (kind) renderedKinds.add(kind);
    }
    // Every NodeKind tag from NodeRenderer's KIND_VISUALS map
    // (the single source of truth for per-kind visuals) shows up.
    for (const tag of NODE_KIND_TAGS) {
      expect(renderedKinds.has(tag)).toBe(true);
    }
  });

  it("clicking a node selects it and surfaces its annotations in the side panel", async () => {
    const user = userEvent.setup();
    const graph: SkillGraph = {
      ...NINE_KIND_GRAPH,
      nodes: NINE_KIND_GRAPH.nodes.map((n) =>
        n.id === "n_action" ? NODE_WITH_ANNOTATIONS : n,
      ),
    };

    render(<SkillCanvas graph={graph} skipFlowRender />);

    expect(screen.getByTestId("canvas-annotations-empty")).toBeInTheDocument();

    await user.click(screen.getByTestId("canvas-node-n_action"));

    const panel = screen.getByTestId("canvas-annotations-panel");
    expect(within(panel).getByTestId("canvas-annotations-node-label")).toHaveTextContent(
      /Send email/,
    );
    expect(within(panel).getByTestId("anno-confidence")).toHaveTextContent("91%");
    expect(within(panel).getByTestId("anno-sample-count")).toHaveTextContent("142");
    expect(within(panel).getByTestId("anno-success-rate")).toHaveTextContent("88%");
    expect(within(panel).getByTestId("anno-policy")).toHaveTextContent(/Autonomous/);
    expect(within(panel).getByTestId("anno-tool-binding")).toHaveTextContent(
      /gmail · messages.send/,
    );
    expect(
      screen.getByTestId("canvas-node-n_action").getAttribute("data-selected"),
    ).toBe("true");
  });

  it("clicking a selected node a second time deselects and returns the panel to its empty hint", async () => {
    const user = userEvent.setup();
    render(<SkillCanvas graph={NINE_KIND_GRAPH} skipFlowRender />);

    const target = screen.getByTestId("canvas-node-n_fetch");
    await user.click(target);
    expect(target.getAttribute("data-selected")).toBe("true");

    await user.click(target);
    expect(screen.getByTestId("canvas-annotations-empty")).toBeInTheDocument();
    expect(target.getAttribute("data-selected")).toBeNull();
  });
});

describe("computeLayout", () => {
  it("places the entry node at the top row and reachable descendants on increasing rows", () => {
    const positions = computeLayout(NINE_KIND_GRAPH);

    const entry = positions.get("n_action");
    expect(entry).toBeDefined();
    expect(entry?.y).toBe(0);

    const condition = positions.get("n_condition");
    const branch = positions.get("n_branch");
    expect(condition?.y).toBeGreaterThan(entry?.y ?? 0);
    expect(branch?.y).toBeGreaterThan(condition?.y ?? 0);

    // Every node has a position (orphans included).
    for (const node of NINE_KIND_GRAPH.nodes) {
      expect(positions.has(node.id)).toBe(true);
    }
  });

  it("parks orphan nodes (unreachable from entry) below the deepest reachable row", () => {
    const graph: SkillGraph = {
      entry: "a",
      nodes: [
        makeNode("a", "A", { type: "action", instruction: "x" }),
        makeNode("b", "B", { type: "action", instruction: "y" }),
        makeNode("orphan", "Orphan", { type: "action", instruction: "z" }),
      ],
      edges: [{ id: "ab", from: "a", to: "b", cond: { kind: "always" } }],
      variables: [],
      metadata: {},
    };

    const positions = computeLayout(graph);
    const reachableMaxY = Math.max(positions.get("a")!.y, positions.get("b")!.y);
    expect(positions.get("orphan")!.y).toBeGreaterThan(reachableMaxY);
  });
});

describe("KIND_VISUALS + EDGE_COND_STROKE", () => {
  it("provides a distinct visual (icon + body class) for every NodeKind variant", () => {
    const seenIcons = new Set<unknown>();
    const seenBodies = new Set<string>();
    for (const tag of NODE_KIND_TAGS) {
      const visual = KIND_VISUALS[tag];
      expect(visual).toBeDefined();
      expect(typeof visual.label).toBe("string");
      expect(typeof visual.bodyClass).toBe("string");
      expect(visual.bodyClass.length).toBeGreaterThan(0);
      seenIcons.add(visual.Icon);
      seenBodies.add(visual.bodyClass);
    }
    expect(seenIcons.size).toBe(NODE_KIND_TAGS.length);
    expect(seenBodies.size).toBe(NODE_KIND_TAGS.length);
  });

  it("maps each EdgeCond kind to a distinct stroke", () => {
    const strokes = new Set(Object.values(EDGE_COND_STROKE));
    expect(strokes.size).toBe(Object.keys(EDGE_COND_STROKE).length);
    expect(EDGE_COND_STROKE["on-success"]).toMatch(/success|#16a34a/);
    expect(EDGE_COND_STROKE["on-failure"]).toMatch(/destructive|#dc2626/);
  });
});

describe("AnnotationsPanel formatters", () => {
  it("formats percent / policy / confidence-tone helpers safely", () => {
    expect(formatPercent(0.876)).toBe("88%");
    expect(formatPercent(undefined)).toBe("—");
    expect(formatPercent(Number.NaN)).toBe("—");

    expect(policyLabel("auto")).toMatch(/Autonomous/);
    expect(policyLabel("ask")).toMatch(/Ask operator/);
    expect(policyLabel(undefined)).toBe("—");

    expect(confidenceTone(0.95)).toBe("ok");
    expect(confidenceTone(0.7)).toBe("warn");
    expect(confidenceTone(0.3)).toBe("bad");
    expect(confidenceTone(undefined)).toBeUndefined();
  });

  it("renders the empty hint when no node is selected, and the body when one is", () => {
    const { rerender } = render(<AnnotationsPanel node={null} />);
    expect(screen.getByTestId("canvas-annotations-empty")).toBeInTheDocument();

    rerender(<AnnotationsPanel node={NODE_WITH_ANNOTATIONS} />);
    expect(screen.queryByTestId("canvas-annotations-empty")).toBeNull();
    expect(screen.getByTestId("canvas-annotations-node-label")).toHaveTextContent(
      /Send email/,
    );
  });
});
