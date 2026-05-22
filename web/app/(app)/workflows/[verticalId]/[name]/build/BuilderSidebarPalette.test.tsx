import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { BuilderSidebarPalette } from "./BuilderSidebarPalette";
import { PALETTE_DRAG_MIME } from "./types";

describe("BuilderSidebarPalette", () => {
  it("renders the six grouped categories from the brief", () => {
    render(<BuilderSidebarPalette onSelect={() => {}} />);
    expect(screen.getByText("Triggers")).toBeInTheDocument();
    expect(screen.getByText("Skills")).toBeInTheDocument();
    expect(screen.getByText("Logic")).toBeInTheDocument();
    expect(screen.getByText("Approval gates")).toBeInTheDocument();
    expect(screen.getByText("Tools")).toBeInTheDocument();
    expect(screen.getByText("Notify")).toBeInTheDocument();
  });

  it("renders sub-items per group (manual / schedule / webhook / email / slack / form triggers)", () => {
    render(<BuilderSidebarPalette onSelect={() => {}} />);
    expect(screen.getByTestId("builder-palette-item-trigger-manual")).toBeInTheDocument();
    expect(screen.getByTestId("builder-palette-item-trigger-schedule")).toBeInTheDocument();
    expect(screen.getByTestId("builder-palette-item-trigger-webhook")).toBeInTheDocument();
    expect(screen.getByTestId("builder-palette-item-trigger-email")).toBeInTheDocument();
    expect(screen.getByTestId("builder-palette-item-trigger-slack")).toBeInTheDocument();
    expect(screen.getByTestId("builder-palette-item-trigger-form")).toBeInTheDocument();
  });

  it("filters items in real time as the user types", () => {
    render(<BuilderSidebarPalette onSelect={() => {}} />);
    const search = screen.getByTestId("builder-palette-search") as HTMLInputElement;
    fireEvent.change(search, { target: { value: "webhook" } });
    expect(screen.getByTestId("builder-palette-item-trigger-webhook")).toBeInTheDocument();
    expect(screen.queryByTestId("builder-palette-item-trigger-manual")).not.toBeInTheDocument();
    expect(screen.queryByTestId("builder-palette-item-skill-invoke")).not.toBeInTheDocument();
  });

  it("shows an empty state when nothing matches the search", () => {
    render(<BuilderSidebarPalette onSelect={() => {}} />);
    const search = screen.getByTestId("builder-palette-search");
    fireEvent.change(search, { target: { value: "zzzzzzzz" } });
    expect(screen.getByTestId("builder-palette-empty")).toBeInTheDocument();
  });

  it("matches on group label and description text, not just item label", () => {
    render(<BuilderSidebarPalette onSelect={() => {}} />);
    const search = screen.getByTestId("builder-palette-search");
    // "marketplace" appears only in the Skill description.
    fireEvent.change(search, { target: { value: "marketplace" } });
    expect(screen.getByTestId("builder-palette-item-skill-invoke")).toBeInTheDocument();
    expect(screen.queryByTestId("builder-palette-item-trigger-manual")).not.toBeInTheDocument();
  });

  it("collapses the whole palette to a rail and re-expands on click", () => {
    render(<BuilderSidebarPalette onSelect={() => {}} />);
    fireEvent.click(screen.getByTestId("builder-palette-collapse"));
    expect(screen.getByTestId("builder-palette-rail")).toBeInTheDocument();
    expect(screen.queryByTestId("builder-palette")).not.toBeInTheDocument();
    fireEvent.click(screen.getByLabelText("Expand palette"));
    expect(screen.getByTestId("builder-palette")).toBeInTheDocument();
  });

  it("collapses an individual group when its header is clicked", () => {
    render(<BuilderSidebarPalette onSelect={() => {}} />);
    expect(screen.getByTestId("builder-palette-item-skill-invoke")).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("builder-palette-group-skills"));
    expect(screen.queryByTestId("builder-palette-item-skill-invoke")).not.toBeInTheDocument();
  });

  it("invokes onSelect with the full PaletteItem when an item is clicked", () => {
    const onSelect = vi.fn();
    render(<BuilderSidebarPalette onSelect={onSelect} />);
    fireEvent.click(screen.getByTestId("builder-palette-item-trigger-schedule"));
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect.mock.calls[0][0]).toMatchObject({
      id: "trigger-schedule",
      kind: "trigger",
      triggerKind: "schedule",
      label: "Schedule",
    });
  });

  it("invokes onSelect when an item receives Enter or Space via keyboard", () => {
    const onSelect = vi.fn();
    render(<BuilderSidebarPalette onSelect={onSelect} />);
    const item = screen.getByTestId("builder-palette-item-notify-email");
    fireEvent.keyDown(item, { key: "Enter" });
    expect(onSelect).toHaveBeenCalledTimes(1);
    fireEvent.keyDown(item, { key: " " });
    expect(onSelect).toHaveBeenCalledTimes(2);
  });

  it("populates the custom MIME-type dataTransfer on drag start", () => {
    render(<BuilderSidebarPalette onSelect={() => {}} />);
    const item = screen.getByTestId("builder-palette-item-tool-http");
    const calls: Array<[string, string]> = [];
    const dataTransfer = {
      setData: (mime: string, payload: string) => calls.push([mime, payload]),
      effectAllowed: "",
    } as unknown as DataTransfer;
    fireEvent.dragStart(item, { dataTransfer });
    expect(calls).toHaveLength(1);
    expect(calls[0][0]).toBe(PALETTE_DRAG_MIME);
    const parsed = JSON.parse(calls[0][1]);
    expect(parsed).toMatchObject({ id: "tool-http", kind: "http_request", label: "HTTP request" });
  });
});
