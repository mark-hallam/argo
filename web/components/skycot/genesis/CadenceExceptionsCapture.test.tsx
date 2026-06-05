import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import {
  CadenceExceptionsCapture,
  EMPTY_CADENCE_EXCEPTIONS,
  type CadenceExceptionsSelection,
} from "./CadenceExceptionsCapture";

/**
 * Genesis UF Phase-A · A1 intake capture panel tests. Presentational + state
 * mechanics; these assert the three typed slots the parent threads into the SSE
 * request: cadence (free text), exceptions (list), success-criteria (typed).
 */

describe("CadenceExceptionsCapture", () => {
  it("captures a cadence string", () => {
    const onChange = vi.fn();
    render(
      <CadenceExceptionsCapture value={EMPTY_CADENCE_EXCEPTIONS} onChange={onChange} />,
    );
    fireEvent.change(screen.getByTestId("genesis-cadence-input"), {
      target: { value: "every weekday at 9am" },
    });
    const next = onChange.mock.calls[0][0] as CadenceExceptionsSelection;
    expect(next.cadence).toBe("every weekday at 9am");
  });

  it("adds an exception to the list", () => {
    const onChange = vi.fn();
    render(
      <CadenceExceptionsCapture value={EMPTY_CADENCE_EXCEPTIONS} onChange={onChange} />,
    );
    fireEvent.change(screen.getByTestId("genesis-exception-input"), {
      target: { value: "never touch legal escalations" },
    });
    fireEvent.click(screen.getByTestId("genesis-exception-add"));
    const next = onChange.mock.calls.at(-1)?.[0] as CadenceExceptionsSelection;
    expect(next.exceptions).toEqual(["never touch legal escalations"]);
  });

  it("does not add a blank exception", () => {
    const onChange = vi.fn();
    render(
      <CadenceExceptionsCapture value={EMPTY_CADENCE_EXCEPTIONS} onChange={onChange} />,
    );
    // The add button is disabled with empty draft; clicking is a no-op.
    expect(screen.getByTestId("genesis-exception-add")).toBeDisabled();
  });

  it("adds a typed success criterion with reused KPI vocabulary", () => {
    const onChange = vi.fn();
    render(
      <CadenceExceptionsCapture value={EMPTY_CADENCE_EXCEPTIONS} onChange={onChange} />,
    );
    fireEvent.click(screen.getByTestId("genesis-criterion-add"));
    const next = onChange.mock.calls[0][0] as CadenceExceptionsSelection;
    expect(next.success_criteria).toHaveLength(1);
    // Defaults mirror the Rust SuccessCriterion wire tokens.
    expect(next.success_criteria[0]).toMatchObject({
      kind: "count",
      comparator: "gte",
      target: 0,
    });
  });

  it("edits a criterion's metric and target", () => {
    let value: CadenceExceptionsSelection = {
      cadence: "",
      exceptions: [],
      success_criteria: [{ name: "", kind: "count", target: 0, comparator: "gte" }],
    };
    const onChange = vi.fn((next: CadenceExceptionsSelection) => {
      value = next;
    });
    render(<CadenceExceptionsCapture value={value} onChange={onChange} />);
    fireEvent.change(screen.getByTestId("genesis-criterion-target-0"), {
      target: { value: "95" },
    });
    const next = onChange.mock.calls.at(-1)?.[0] as CadenceExceptionsSelection;
    expect(next.success_criteria[0].target).toBe(95);
  });

  it("removes a criterion", () => {
    const value: CadenceExceptionsSelection = {
      cadence: "",
      exceptions: [],
      success_criteria: [{ name: "x", kind: "count", target: 1, comparator: "gte" }],
    };
    const onChange = vi.fn();
    render(<CadenceExceptionsCapture value={value} onChange={onChange} />);
    fireEvent.click(screen.getByTestId("genesis-criterion-remove-0"));
    const next = onChange.mock.calls.at(-1)?.[0] as CadenceExceptionsSelection;
    expect(next.success_criteria).toHaveLength(0);
  });
});
