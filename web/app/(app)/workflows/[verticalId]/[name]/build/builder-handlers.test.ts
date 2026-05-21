import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";

import {
  isValidConnectionShape,
  newStepNode,
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
