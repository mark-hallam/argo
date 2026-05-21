import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Node } from "@xyflow/react";

import { useBuilderAnnotations } from "./use-builder-annotations";

import type { SkyNodeData } from "@/app/(app)/workflows/[verticalId]/[name]/build/types";

function stepNode(id: string): Node {
  const data: SkyNodeData = {
    step: { id, kind: "skill_invoke", name: id, description: "" },
  };
  return { id, type: "skycot", position: { x: 0, y: 0 }, data };
}

describe("useBuilderAnnotations — sticky notes", () => {
  it("addNote adds an empty note at the requested position", () => {
    const { result } = renderHook(() => useBuilderAnnotations());
    let id = "";
    act(() => {
      id = result.current.addNote(120, 340);
    });
    expect(result.current.notes).toHaveLength(1);
    expect(result.current.notes[0]).toMatchObject({ id, text: "", x: 120, y: 340 });
    expect(result.current.noteNodes[0]).toMatchObject({
      id,
      type: "sticky",
      position: { x: 120, y: 340 },
    });
    expect(result.current.isNoteId(id)).toBe(true);
  });

  it("updateNoteText replaces the text without moving the note", () => {
    const { result } = renderHook(() => useBuilderAnnotations());
    let id = "";
    act(() => {
      id = result.current.addNote(0, 0);
    });
    act(() => {
      result.current.updateNoteText(id, "remember to wire pin to gateway");
    });
    expect(result.current.notes[0]?.text).toBe("remember to wire pin to gateway");
  });

  it("updateNotePosition moves the note", () => {
    const { result } = renderHook(() => useBuilderAnnotations());
    let id = "";
    act(() => {
      id = result.current.addNote(0, 0);
    });
    act(() => {
      result.current.updateNotePosition(id, 200, 400);
    });
    expect(result.current.notes[0]).toMatchObject({ x: 200, y: 400 });
  });

  it("deleteNote removes the note from notes and noteNodes", () => {
    const { result } = renderHook(() => useBuilderAnnotations());
    let id = "";
    act(() => {
      id = result.current.addNote(0, 0);
    });
    expect(result.current.notes).toHaveLength(1);
    act(() => {
      result.current.deleteNote(id);
    });
    expect(result.current.notes).toHaveLength(0);
    expect(result.current.noteNodes).toHaveLength(0);
    expect(result.current.isNoteId(id)).toBe(false);
  });
});

describe("useBuilderAnnotations — pin data", () => {
  it("savePin stores JSON text keyed by step id", () => {
    const { result } = renderHook(() => useBuilderAnnotations());
    act(() => {
      result.current.savePin("step-a", '{"x": 1}');
    });
    expect(result.current.pinnedData["step-a"]).toBe('{"x": 1}');
  });

  it("clearPin removes the entry", () => {
    const { result } = renderHook(() => useBuilderAnnotations());
    act(() => {
      result.current.savePin("step-a", '{"x": 1}');
    });
    act(() => {
      result.current.clearPin("step-a");
    });
    expect(result.current.pinnedData["step-a"]).toBeUndefined();
    expect(Object.keys(result.current.pinnedData)).toHaveLength(0);
  });

  it("clearPin is a no-op when the id has no pin", () => {
    const { result } = renderHook(() => useBuilderAnnotations());
    const before = result.current.pinnedData;
    act(() => {
      result.current.clearPin("never-pinned");
    });
    expect(result.current.pinnedData).toBe(before);
  });
});

describe("useBuilderAnnotations — decorateWithPins", () => {
  it("returns the input array unchanged when no pins exist", () => {
    const { result } = renderHook(() => useBuilderAnnotations());
    const nodes = [stepNode("a"), stepNode("b")];
    const decorated = result.current.decorateWithPins(nodes);
    expect(decorated).toBe(nodes);
  });

  it("projects pinned=true onto the matching step node only", () => {
    const { result } = renderHook(() => useBuilderAnnotations());
    act(() => {
      result.current.savePin("a", '{"x": 1}');
    });
    const decorated = result.current.decorateWithPins([
      stepNode("a"),
      stepNode("b"),
    ]);
    expect((decorated[0].data as SkyNodeData).pinned).toBe(true);
    expect((decorated[1].data as SkyNodeData).pinned).toBeUndefined();
  });

  it("clears the pinned flag once the pin is unpinned", () => {
    const { result } = renderHook(() => useBuilderAnnotations());
    act(() => {
      result.current.savePin("a", '{"x": 1}');
    });
    const stillPinned = result.current.decorateWithPins([stepNode("a")]);
    expect((stillPinned[0].data as SkyNodeData).pinned).toBe(true);
    act(() => {
      result.current.clearPin("a");
    });
    const cleared = result.current.decorateWithPins([stepNode("a")]);
    expect((cleared[0].data as SkyNodeData).pinned).toBeUndefined();
  });
});
