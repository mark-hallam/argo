import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CommandPaletteProvider } from "./CommandPaletteProvider";

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), refresh: vi.fn() }),
}));

function ProbeInput() {
  return (
    <input
      data-testid="probe-input"
      type="text"
      onKeyDown={(e) => {
        // Mirror the hero-search defensive handler under test.
        if (!(e.metaKey || e.ctrlKey)) e.stopPropagation();
      }}
    />
  );
}

describe("CommandPaletteProvider — focus contention defense", () => {
  it("typing alphanumeric keys into a focused input does NOT open the palette", () => {
    render(
      <CommandPaletteProvider>
        <ProbeInput />
      </CommandPaletteProvider>,
    );

    const input = screen.getByTestId("probe-input") as HTMLInputElement;
    input.focus();

    // Simulate a 10-character query at the synthetic event speed that the
    // Claude Preview MCP eval tool uses. None of these letters are the chord
    // (k must be paired with metaKey/ctrlKey). The palette must stay closed.
    for (const key of ["a", "p", "p", "r", "o", "v", "a", "l", "s", "k"]) {
      fireEvent.keyDown(input, { key });
      fireEvent.keyUp(input, { key });
    }

    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("a bare 'k' keystroke (no modifier) does NOT open the palette", () => {
    render(
      <CommandPaletteProvider>
        <div data-testid="background" />
      </CommandPaletteProvider>,
    );

    fireEvent.keyDown(window, { key: "k" });
    fireEvent.keyUp(window, { key: "k" });

    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("the ⌘K chord still opens the palette (no regression)", () => {
    render(
      <CommandPaletteProvider>
        <div data-testid="background" />
      </CommandPaletteProvider>,
    );

    fireEvent.keyDown(window, { key: "k", metaKey: true });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("Ctrl+K chord also opens the palette (Windows/Linux)", () => {
    render(
      <CommandPaletteProvider>
        <div data-testid="background" />
      </CommandPaletteProvider>,
    );

    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("a defaultPrevented chord event is ignored (inner handler already handled it)", () => {
    render(
      <CommandPaletteProvider>
        <div data-testid="background" />
      </CommandPaletteProvider>,
    );

    // Fire a ⌘K with defaultPrevented = true.
    const ev = new KeyboardEvent("keydown", {
      key: "k",
      metaKey: true,
      bubbles: true,
      cancelable: true,
    });
    ev.preventDefault();
    window.dispatchEvent(ev);

    expect(screen.queryByRole("dialog")).toBeNull();
  });
});
