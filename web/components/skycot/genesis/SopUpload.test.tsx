import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SopUpload, type SopSelection } from "./SopUpload";

/**
 * Genesis P6 SOP-upload control tests. The component is presentational + file
 * mechanics; these assert the moat-input behaviour the parent relies on:
 * attaching a text file yields a `text` selection, the chip + remove work, and
 * the optional control never blocks the text-only path.
 */

describe("SopUpload", () => {
  it("starts with no selection and an optional attach affordance", () => {
    render(<SopUpload value={null} onChange={() => {}} />);
    expect(screen.getByTestId("genesis-sop-attach")).toBeInTheDocument();
    // The copy must be honest that it's optional + that comprehension is checked.
    expect(screen.getByTestId("genesis-sop-attach").textContent).toMatch(/optional/i);
  });

  it("reads an attached .txt as a text selection", async () => {
    const onChange = vi.fn();
    render(<SopUpload value={null} onChange={onChange} />);
    const input = screen.getByTestId("genesis-sop-file-input") as HTMLInputElement;
    const file = new File(["Escalate refunds above the threshold to Priyanka."], "policy.txt", {
      type: "text/plain",
    });
    fireEvent.change(input, { target: { files: [file] } });
    await waitFor(() => expect(onChange).toHaveBeenCalled());
    const sel = onChange.mock.calls[0][0] as SopSelection;
    expect(sel.format).toBe("text");
    expect(sel.filename).toBe("policy.txt");
    expect(sel.document).toContain("Priyanka");
  });

  it("renders the attached chip + quiz hint when a selection is present", () => {
    const value: SopSelection = {
      document: "x".repeat(100),
      format: "text",
      filename: "runbook.md",
    };
    render(<SopUpload value={value} onChange={() => {}} />);
    expect(screen.getByTestId("genesis-sop-chip").textContent).toContain("runbook.md");
    // The moat promise is surfaced: the teammate will be quizzed on it.
    expect(screen.getByTestId("genesis-sop-chip").parentElement?.textContent).toMatch(
      /quizzed on this before it can be hired/i,
    );
  });

  it("clears the selection via the remove button", () => {
    const onChange = vi.fn();
    const value: SopSelection = { document: "y".repeat(100), format: "text", filename: "a.txt" };
    render(<SopUpload value={value} onChange={onChange} />);
    fireEvent.click(screen.getByTestId("genesis-sop-remove"));
    expect(onChange).toHaveBeenCalledWith(null);
  });

  it("rejects an empty text file rather than attaching an unquizzable SOP", async () => {
    const onChange = vi.fn();
    render(<SopUpload value={null} onChange={onChange} />);
    const input = screen.getByTestId("genesis-sop-file-input") as HTMLInputElement;
    const empty = new File(["   \n  "], "blank.txt", { type: "text/plain" });
    fireEvent.change(input, { target: { files: [empty] } });
    await waitFor(() => expect(screen.getByTestId("genesis-sop-error")).toBeInTheDocument());
    expect(onChange).not.toHaveBeenCalled();
  });
});
