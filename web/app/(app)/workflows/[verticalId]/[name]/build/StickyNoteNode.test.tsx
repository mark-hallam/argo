import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { NodeProps } from "@xyflow/react";

import { StickyNoteNode } from "./StickyNoteNode";
import type { SkyNoteData } from "./types";

function makeProps(over: Partial<SkyNoteData> = {}) {
  const data: SkyNoteData = {
    noteId: "note-1",
    text: "",
    onTextChange: vi.fn(),
    onDelete: vi.fn(),
    ...over,
  };
  return { data } as unknown as NodeProps;
}

describe("StickyNoteNode", () => {
  it("renders the placeholder when text is empty", () => {
    render(<StickyNoteNode {...makeProps()} />);
    expect(screen.getByText("Double-click to edit")).toBeInTheDocument();
  });

  it("renders the note text when present", () => {
    render(<StickyNoteNode {...makeProps({ text: "wire to gateway" })} />);
    expect(screen.getByText("wire to gateway")).toBeInTheDocument();
  });

  it("enters edit mode on double-click and commits on blur", () => {
    const onTextChange = vi.fn();
    render(<StickyNoteNode {...makeProps({ onTextChange })} />);
    const sticky = screen.getByTestId("sticky-note-note-1");
    fireEvent.doubleClick(sticky);
    const input = screen.getByTestId("sticky-note-input-note-1") as HTMLTextAreaElement;
    fireEvent.change(input, { target: { value: "hello world" } });
    fireEvent.blur(input);
    expect(onTextChange).toHaveBeenCalledWith("note-1", "hello world");
  });

  it("Escape cancels the edit without committing", () => {
    const onTextChange = vi.fn();
    render(<StickyNoteNode {...makeProps({ text: "before", onTextChange })} />);
    fireEvent.doubleClick(screen.getByTestId("sticky-note-note-1"));
    const input = screen.getByTestId("sticky-note-input-note-1") as HTMLTextAreaElement;
    fireEvent.change(input, { target: { value: "after" } });
    fireEvent.keyDown(input, { key: "Escape" });
    expect(onTextChange).not.toHaveBeenCalled();
    expect(screen.getByText("before")).toBeInTheDocument();
  });

  it("calls onDelete when the × button is clicked", () => {
    const onDelete = vi.fn();
    render(<StickyNoteNode {...makeProps({ onDelete })} />);
    fireEvent.click(screen.getByTestId("sticky-note-delete-note-1"));
    expect(onDelete).toHaveBeenCalledWith("note-1");
  });

  it("does not re-commit if the text was not changed", () => {
    const onTextChange = vi.fn();
    render(<StickyNoteNode {...makeProps({ text: "same", onTextChange })} />);
    fireEvent.doubleClick(screen.getByTestId("sticky-note-note-1"));
    const input = screen.getByTestId("sticky-note-input-note-1") as HTMLTextAreaElement;
    fireEvent.blur(input);
    expect(onTextChange).not.toHaveBeenCalled();
  });
});
