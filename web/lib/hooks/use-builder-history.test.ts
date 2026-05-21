import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";

import { useBuilderHistory, HISTORY_LIMIT, type BuilderSnapshot } from "./use-builder-history";

function snap(label: string, nodes: Node[] = [], edges: Edge[] = []): BuilderSnapshot {
  return { label, nodes, edges, takenAt: Date.now() };
}

describe("useBuilderHistory", () => {
  it("push then undo returns the pushed snapshot", () => {
    const { result } = renderHook(() => useBuilderHistory());
    expect(result.current.canUndo()).toBe(false);
    act(() => {
      result.current.push(snap("Add step"));
    });
    expect(result.current.canUndo()).toBe(true);
    let restored: BuilderSnapshot | null = null;
    act(() => {
      restored = result.current.undo() as BuilderSnapshot | null;
    });
    expect((restored as BuilderSnapshot | null)?.label).toBe("Add step");
    expect(result.current.canUndo()).toBe(false);
  });

  it("undo returns null when history is empty", () => {
    const { result } = renderHook(() => useBuilderHistory());
    expect(result.current.undo()).toBeNull();
    expect(result.current.redo()).toBeNull();
  });

  it("push clears any future entries (post-undo divergence)", () => {
    const { result } = renderHook(() => useBuilderHistory());
    act(() => {
      result.current.push(snap("A"));
      result.current.pushFuture(snap("B"));
    });
    expect(result.current.canRedo()).toBe(true);
    act(() => {
      result.current.push(snap("C"));
    });
    expect(result.current.canRedo()).toBe(false);
  });

  it("caps history at HISTORY_LIMIT entries (oldest dropped)", () => {
    const { result } = renderHook(() => useBuilderHistory());
    act(() => {
      for (let i = 0; i < HISTORY_LIMIT + 5; i++) {
        result.current.push(snap(`s${i}`));
      }
    });
    let popped: BuilderSnapshot | null = null;
    act(() => {
      popped = result.current.undo() as BuilderSnapshot | null;
    });
    // The most recent push wins; the oldest 5 were dropped.
    expect((popped as BuilderSnapshot | null)?.label).toBe(`s${HISTORY_LIMIT + 4}`);
  });

  it("clear empties both stacks", () => {
    const { result } = renderHook(() => useBuilderHistory());
    act(() => {
      result.current.push(snap("A"));
      result.current.pushFuture(snap("B"));
    });
    expect(result.current.canUndo()).toBe(true);
    expect(result.current.canRedo()).toBe(true);
    act(() => {
      result.current.clear();
    });
    expect(result.current.canUndo()).toBe(false);
    expect(result.current.canRedo()).toBe(false);
  });
});
