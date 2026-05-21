import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { PinDataModal } from "./PinDataModal";

function setup(props: Partial<Parameters<typeof PinDataModal>[0]> = {}) {
  const onSave = vi.fn();
  const onClear = vi.fn();
  const onClose = vi.fn();
  render(
    <PinDataModal
      stepId="step-a"
      stepName="Fetch customer"
      initialJson={null}
      onSave={onSave}
      onClear={onClear}
      onClose={onClose}
      {...props}
    />,
  );
  return { onSave, onClear, onClose };
}

describe("PinDataModal", () => {
  it("renders the step name and id in the header", () => {
    setup();
    expect(screen.getByText(/Fetch customer/)).toBeInTheDocument();
    expect(screen.getByText(/step-a/)).toBeInTheDocument();
  });

  it("Save button is disabled when the textarea is invalid JSON", () => {
    setup();
    const textarea = screen.getByTestId("pin-data-textarea") as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: "{ broken" } });
    expect(screen.getByTestId("pin-data-error")).toBeInTheDocument();
    expect(screen.getByTestId("pin-data-save")).toBeDisabled();
  });

  it("calls onSave with pretty-printed JSON on Save", () => {
    const { onSave } = setup();
    const textarea = screen.getByTestId("pin-data-textarea") as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: '{"x":1,"y":2}' } });
    fireEvent.click(screen.getByTestId("pin-data-save"));
    expect(onSave).toHaveBeenCalledTimes(1);
    const saved = onSave.mock.calls[0]?.[0] as string;
    expect(saved).toBe('{\n  "x": 1,\n  "y": 2\n}');
  });

  it("Unpin is disabled when no pin exists", () => {
    setup();
    expect(screen.getByTestId("pin-data-unpin")).toBeDisabled();
  });

  it("Unpin is enabled and fires onClear when a pin exists", () => {
    const { onClear, onClose } = setup({ initialJson: '{"x": 1}' });
    const unpin = screen.getByTestId("pin-data-unpin");
    expect(unpin).not.toBeDisabled();
    fireEvent.click(unpin);
    expect(onClear).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("Escape closes the modal", () => {
    const { onClose } = setup();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });
});
