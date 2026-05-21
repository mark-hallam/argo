import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Edge, Node, NodeChange } from "@xyflow/react";
import { useWorkflowBuilderState } from "./use-workflow-builder-state";

function n(id: string, x = 0, y = 0): Node {
  return { id, position: { x, y }, data: {} };
}

function e(id: string, source: string, target: string): Edge {
  return { id, source, target };
}

describe("useWorkflowBuilderState", () => {
  it("seeds nodes and edges from the initial arrays", () => {
    const initialNodes = [n("a"), n("b")];
    const initialEdges = [e("a-b", "a", "b")];
    const { result } = renderHook(() =>
      useWorkflowBuilderState(initialNodes, initialEdges),
    );
    expect(result.current.nodes.map((node) => node.id)).toEqual(["a", "b"]);
    expect(result.current.edges.map((edge) => edge.id)).toEqual(["a-b"]);
    expect(result.current.dirty).toBe(false);
  });

  it("onNodesChange applies position updates via applyNodeChanges", () => {
    const { result } = renderHook(() =>
      useWorkflowBuilderState([n("a", 0, 0)], []),
    );
    const change: NodeChange = {
      id: "a",
      type: "position",
      position: { x: 150, y: 75 },
      dragging: false,
    };
    act(() => {
      result.current.onNodesChange([change]);
    });
    expect(result.current.nodes[0]?.position).toEqual({ x: 150, y: 75 });
    expect(result.current.dirty).toBe(true);
  });

  it("addNode appends new nodes and dedupes by id", () => {
    const { result } = renderHook(() => useWorkflowBuilderState([n("a")], []));
    act(() => {
      result.current.addNode(n("b"));
      result.current.addNode(n("b"));
    });
    expect(result.current.nodes.map((node) => node.id)).toEqual(["a", "b"]);
  });

  it("removeNode drops the node and cascades to incident edges", () => {
    const { result } = renderHook(() =>
      useWorkflowBuilderState(
        [n("a"), n("b"), n("c")],
        [e("a-b", "a", "b"), e("b-c", "b", "c"), e("a-c", "a", "c")],
      ),
    );
    act(() => {
      result.current.removeNode("b");
    });
    expect(result.current.nodes.map((node) => node.id)).toEqual(["a", "c"]);
    expect(result.current.edges.map((edge) => edge.id)).toEqual(["a-c"]);
  });

  it("addEdge appends and removeEdge filters by id (no orphan-edge cleanup beyond removeNode)", () => {
    const { result } = renderHook(() =>
      useWorkflowBuilderState([n("a"), n("b")], []),
    );
    act(() => {
      result.current.addEdge(e("a-b", "a", "b"));
      result.current.addEdge(e("a-b", "a", "b"));
    });
    expect(result.current.edges.map((edge) => edge.id)).toEqual(["a-b"]);
    act(() => {
      result.current.removeEdge("a-b");
    });
    expect(result.current.edges).toEqual([]);
  });

  it("dirty flips true after a setter mutation and false again after reset to the initial shape", () => {
    const initialNodes = [n("a")];
    const initialEdges: Edge[] = [];
    const { result } = renderHook(() =>
      useWorkflowBuilderState(initialNodes, initialEdges),
    );
    expect(result.current.dirty).toBe(false);
    act(() => {
      result.current.addNode(n("b"));
    });
    expect(result.current.dirty).toBe(true);
    act(() => {
      result.current.reset(initialNodes, initialEdges);
    });
    expect(result.current.dirty).toBe(false);
  });
});
