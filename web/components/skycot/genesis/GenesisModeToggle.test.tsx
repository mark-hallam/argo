import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { GenesisModeToggle } from "./GenesisModeToggle";

describe("GenesisModeToggle", () => {
  it("marks the active mode as selected", () => {
    render(<GenesisModeToggle mode="describe" onChange={vi.fn()} />);
    expect(screen.getByTestId("genesis-mode-describe")).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByTestId("genesis-mode-talk")).toHaveAttribute(
      "aria-selected",
      "false",
    );
  });

  it("switches to talk mode on click", () => {
    const onChange = vi.fn();
    render(<GenesisModeToggle mode="describe" onChange={onChange} />);
    fireEvent.click(screen.getByTestId("genesis-mode-talk"));
    expect(onChange).toHaveBeenCalledWith("talk");
  });

  it("disables both buttons when disabled", () => {
    render(<GenesisModeToggle mode="talk" onChange={vi.fn()} disabled />);
    expect(screen.getByTestId("genesis-mode-describe")).toBeDisabled();
    expect(screen.getByTestId("genesis-mode-talk")).toBeDisabled();
  });
});
