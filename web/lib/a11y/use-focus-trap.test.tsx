import { fireEvent, render, screen } from "@testing-library/react";
import { useRef } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useFocusTrap } from "./use-focus-trap";

/**
 * Minimal harness: a button that lives OUTSIDE the dialog (so we can
 * assert focus restoration to it), plus a dialog with two focusable
 * children driven by the hook.
 */
function Harness({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  return (
    <>
      <button data-testid="opener">Opener</button>
      {open ? <Dialog onClose={onClose} /> : null}
    </>
  );
}

function Dialog({ onClose }: { onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  useFocusTrap(ref, onClose);
  return (
    <div
      ref={ref}
      role="dialog"
      aria-modal="true"
      aria-label="Test dialog"
      tabIndex={-1}
      data-testid="dialog"
    >
      <button data-testid="first">First</button>
      <button data-testid="last">Last</button>
    </div>
  );
}

function EmptyDialog({ onClose }: { onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  useFocusTrap(ref, onClose);
  return (
    <div
      ref={ref}
      role="dialog"
      aria-modal="true"
      aria-label="Empty dialog"
      tabIndex={-1}
      data-testid="empty-dialog"
    >
      <p>No focusable children here.</p>
    </div>
  );
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("useFocusTrap", () => {
  it("moves focus to the first focusable child on open", () => {
    render(<Harness open onClose={() => {}} />);
    expect(document.activeElement).toBe(screen.getByTestId("first"));
  });

  it("falls back to focusing the dialog when there are no focusable children", () => {
    render(<EmptyDialog onClose={() => {}} />);
    expect(document.activeElement).toBe(screen.getByTestId("empty-dialog"));
  });

  it("calls onClose on Escape", () => {
    const onClose = vi.fn();
    render(<Harness open onClose={onClose} />);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("wraps Tab from the last element back to the first", () => {
    render(<Harness open onClose={() => {}} />);
    const last = screen.getByTestId("last");
    const first = screen.getByTestId("first");
    last.focus();
    fireEvent.keyDown(document, { key: "Tab" });
    expect(document.activeElement).toBe(first);
  });

  it("wraps Shift+Tab from the first element to the last", () => {
    render(<Harness open onClose={() => {}} />);
    const first = screen.getByTestId("first");
    const last = screen.getByTestId("last");
    first.focus();
    fireEvent.keyDown(document, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(last);
  });

  it("restores focus to the previously-focused element when the dialog closes", () => {
    const { rerender } = render(<Harness open={false} onClose={() => {}} />);
    const opener = screen.getByTestId("opener");
    opener.focus();
    expect(document.activeElement).toBe(opener);

    // Open: focus moves into the dialog.
    rerender(<Harness open onClose={() => {}} />);
    expect(document.activeElement).toBe(screen.getByTestId("first"));

    // Close: focus returns to the opener.
    rerender(<Harness open={false} onClose={() => {}} />);
    expect(document.activeElement).toBe(opener);
  });

  it("always invokes the latest onClose (no stale closure)", () => {
    const firstHandler = vi.fn();
    const secondHandler = vi.fn();
    const { rerender } = render(<Harness open onClose={firstHandler} />);
    // Re-render with a new handler identity without remounting the dialog.
    rerender(<Harness open onClose={secondHandler} />);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(firstHandler).not.toHaveBeenCalled();
    expect(secondHandler).toHaveBeenCalledTimes(1);
  });
});
