import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";

import {
  findNearestPredecessor,
  isValidConnectionShape,
  newStepNode,
  newStepNodeAt,
  newStepPosition,
  validateConnection,
} from "./builder-handlers";
import type { SkyNodeData, StepKind } from "./types";

function n(id: string, kind: StepKind = "skill_invoke", x = 0, y = 0): Node {
  const data: SkyNodeData = {
    step: { id, kind, name: id, description: "" },
  };
  return { id, position: { x, y }, data };
}

function e(id: string, source: string, target: string): Edge {
  return { id, source, target };
}

describe("validateConnection", () => {
  it("rejects a self-loop", () => {
    const nodes = [n("a"), n("b")];
    const result = validateConnection(
      { source: "a", target: "a", sourceHandle: null, targetHandle: null },
      nodes,
      [],
    );
    expect(result).toEqual({ ok: false, reason: "self_loop" });
  });

  it("rejects a duplicate edge already wired source→target", () => {
    const nodes = [n("a"), n("b")];
    const edges = [e("e-a-b", "a", "b")];
    const result = validateConnection(
      { source: "a", target: "b", sourceHandle: null, targetHandle: null },
      nodes,
      edges,
    );
    expect(result).toEqual({ ok: false, reason: "duplicate" });
  });

  it("rejects an incoming edge into a trigger node", () => {
    const nodes = [n("a"), n("t", "trigger")];
    const result = validateConnection(
      { source: "a", target: "t", sourceHandle: null, targetHandle: null },
      nodes,
      [],
    );
    expect(result).toEqual({ ok: false, reason: "trigger_incoming" });
  });

  it("accepts a clean connection and produces a deterministic edge id", () => {
    const nodes = [n("a"), n("b")];
    const result = validateConnection(
      { source: "a", target: "b", sourceHandle: null, targetHandle: null },
      nodes,
      [],
    );
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.edge.id).toBe("e-a-b");
      expect(result.edge.source).toBe("a");
      expect(result.edge.target).toBe("b");
      expect(result.edge.animated).toBe(false);
    }
  });

  it("rejects incomplete connections missing source or target", () => {
    expect(
      validateConnection(
        { source: "", target: "b", sourceHandle: null, targetHandle: null },
        [],
        [],
      ),
    ).toEqual({ ok: false, reason: "incomplete" });
    expect(
      validateConnection(
        { source: "a", target: "", sourceHandle: null, targetHandle: null },
        [],
        [],
      ),
    ).toEqual({ ok: false, reason: "incomplete" });
  });
});

describe("isValidConnectionShape", () => {
  it("returns false when the target is a trigger", () => {
    expect(
      isValidConnectionShape(
        { source: "a", target: "t", sourceHandle: null, targetHandle: null },
        [n("a"), n("t", "trigger")],
      ),
    ).toBe(false);
  });

  it("returns false on self-loop or unknown ids", () => {
    expect(
      isValidConnectionShape(
        { source: "a", target: "a", sourceHandle: null, targetHandle: null },
        [n("a")],
      ),
    ).toBe(false);
  });

  it("returns true for a normal forward connection", () => {
    expect(
      isValidConnectionShape(
        { source: "a", target: "b", sourceHandle: null, targetHandle: null },
        [n("a"), n("b")],
      ),
    ).toBe(true);
  });
});

describe("newStepPosition", () => {
  it("offsets from the last node", () => {
    expect(newStepPosition([n("a", "skill_invoke", 100, 200)])).toEqual({
      x: 320,
      y: 260,
    });
  });

  it("defaults to (200, 200) for an empty canvas", () => {
    expect(newStepPosition([])).toEqual({ x: 200, y: 200 });
  });
});

describe("newStepNode", () => {
  it("creates a skycot-typed node with default name + description for the kind", () => {
    const node = newStepNode("approval", [], "abc12345");
    expect(node.id).toBe("step-abc12345");
    expect(node.type).toBe("skycot");
    expect(node.position).toEqual({ x: 200, y: 200 });
    const data = node.data as SkyNodeData;
    expect(data.step.kind).toBe("approval");
    expect(data.step.name).toBe("Approval gate");
    expect(data.step.description).toBe("Request human review");
  });

  it("offsets position from the last existing node", () => {
    const node = newStepNode(
      "tool_call",
      [n("a", "skill_invoke", 500, 100)],
      "deadbeef",
    );
    expect(node.position).toEqual({ x: 720, y: 160 });
  });
});

describe("newStepNodeAt", () => {
  it("places the new node at the provided position", () => {
    const node = newStepNodeAt("skill_invoke", "abc", { x: 420, y: 800 });
    expect(node.position).toEqual({ x: 420, y: 800 });
    expect(node.id).toBe("step-abc");
    expect(node.type).toBe("skycot");
  });

  it("applies name + description + triggerKind overrides", () => {
    const node = newStepNodeAt("trigger", "x", { x: 0, y: 0 }, {
      name: "Schedule",
      description: "Cron-style recurring trigger",
      triggerKind: "schedule",
    });
    const data = node.data as SkyNodeData;
    expect(data.step.name).toBe("Schedule");
    expect(data.step.description).toBe("Cron-style recurring trigger");
    expect(data.step.triggerKind).toBe("schedule");
  });

  it("omits triggerKind when not provided", () => {
    const node = newStepNodeAt("notify", "y", { x: 0, y: 0 }, { name: "Slack DM", description: "" });
    const data = node.data as SkyNodeData;
    expect(data.step.triggerKind).toBeUndefined();
  });

  it("uses the subWorkflow node type for sub_workflow kind", () => {
    const node = newStepNodeAt("sub_workflow", "sw", { x: 10, y: 20 });
    expect(node.type).toBe("subWorkflow");
  });
});

describe("findNearestPredecessor", () => {
  it("returns the node with the largest y strictly less than dropY", () => {
    const nodes = [
      n("a", "trigger", 100, 60),
      n("b", "skill_invoke", 100, 260),
      n("c", "approval", 100, 460),
    ];
    expect(findNearestPredecessor(nodes, 500)?.id).toBe("c");
    expect(findNearestPredecessor(nodes, 300)?.id).toBe("b");
    expect(findNearestPredecessor(nodes, 200)?.id).toBe("a");
  });

  it("returns null when the drop is above every node or on an empty canvas", () => {
    expect(findNearestPredecessor([], 100)).toBeNull();
    const nodes = [n("a", "trigger", 100, 60)];
    expect(findNearestPredecessor(nodes, 30)).toBeNull();
  });

  it("ignores nodes at the same y as the drop", () => {
    const nodes = [n("a", "trigger", 100, 200), n("b", "skill_invoke", 100, 200)];
    expect(findNearestPredecessor(nodes, 200)).toBeNull();
  });
});
