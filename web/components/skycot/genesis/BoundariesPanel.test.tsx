import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import {
  BoundariesPanel,
  EMPTY_BOUNDARIES,
  boundariesSmartDefaults,
  type BoundariesEcho,
  type BoundariesSpec,
} from "./BoundariesPanel";

/**
 * Genesis Phase-2 "Set boundaries" panel tests
 * (mission-genesis-access-controls-and-guardrails-spec-ui). Cover the three
 * sections (access toggles / threshold slider / escalation topics), the
 * deterministic smart-default mapping, and the no-theater APPLIED echo render.
 */

describe("BoundariesPanel — edit mode", () => {
  it("toggles a data category to read-write", () => {
    const onChange = vi.fn();
    render(<BoundariesPanel value={EMPTY_BOUNDARIES} onChange={onChange} />);
    fireEvent.click(
      screen.getByTestId("genesis-boundary-access-financial-read_write"),
    );
    const next = onChange.mock.calls.at(-1)?.[0] as BoundariesSpec;
    const financial = next.access_scopes.find((s) => s.category === "financial");
    expect(financial?.access).toBe("read_write");
  });

  it("enables the approval threshold and sets an amount via the slider", () => {
    let value: BoundariesSpec = EMPTY_BOUNDARIES;
    const onChange = vi.fn((next: BoundariesSpec) => {
      value = next;
    });
    const { rerender } = render(
      <BoundariesPanel value={value} onChange={onChange} />,
    );
    // The slider is hidden until the threshold is enabled.
    expect(screen.queryByTestId("genesis-boundary-threshold-slider")).toBeNull();
    fireEvent.click(screen.getByTestId("genesis-boundary-threshold-enabled"));
    expect((onChange.mock.calls.at(-1)?.[0] as BoundariesSpec).approval_threshold_usd)
      .not.toBeNull();
    rerender(<BoundariesPanel value={value} onChange={onChange} />);
    fireEvent.change(screen.getByTestId("genesis-boundary-threshold-slider"), {
      target: { value: "1500" },
    });
    expect((onChange.mock.calls.at(-1)?.[0] as BoundariesSpec).approval_threshold_usd)
      .toBe(1500);
  });

  it("adds and removes an escalation topic", () => {
    let value: BoundariesSpec = EMPTY_BOUNDARIES;
    const onChange = vi.fn((next: BoundariesSpec) => {
      value = next;
    });
    const { rerender } = render(
      <BoundariesPanel value={value} onChange={onChange} />,
    );
    fireEvent.change(screen.getByTestId("genesis-boundary-topic-input"), {
      target: { value: "legal" },
    });
    fireEvent.click(screen.getByTestId("genesis-boundary-topic-add"));
    expect((onChange.mock.calls.at(-1)?.[0] as BoundariesSpec).escalation_topics)
      .toEqual(["legal"]);
    rerender(<BoundariesPanel value={value} onChange={onChange} />);
    fireEvent.click(screen.getByTestId("genesis-boundary-topic-remove-0"));
    expect((onChange.mock.calls.at(-1)?.[0] as BoundariesSpec).escalation_topics)
      .toEqual([]);
  });

  it("does not add a blank topic", () => {
    render(<BoundariesPanel value={EMPTY_BOUNDARIES} onChange={vi.fn()} />);
    expect(screen.getByTestId("genesis-boundary-topic-add")).toBeDisabled();
  });
});

describe("boundariesSmartDefaults", () => {
  it("pre-fills customer-success: financial read-only + refund cap", () => {
    const spec = boundariesSmartDefaults("Customer success teammate for renewals");
    const financial = spec.access_scopes.find((s) => s.category === "financial");
    expect(financial?.access).toBe("read_only");
    expect(spec.approval_threshold_usd).not.toBeNull();
  });

  it("pre-fills three distinct archetypes", () => {
    expect(boundariesSmartDefaults("Sales SDR").approval_threshold_usd).toBe(1000);
    expect(
      boundariesSmartDefaults("Marketplace seller assistant").approval_threshold_usd,
    ).toBe(250);
    expect(
      boundariesSmartDefaults("novel role with no keyword").approval_threshold_usd,
    ).toBeNull();
  });
});

describe("BoundariesPanel — applied (no-theater) mode", () => {
  it("renders the server echo's gated classes, not local form state", () => {
    const echo: BoundariesEcho = {
      policy_id: "genesis-boundaries",
      gated_classes: ["financial", "legal"],
      approval_threshold_usd: 500,
      escalation_topics: ["legal"],
      access_scopes: [{ category: "financial", access: "read_only" }],
    };
    render(<BoundariesPanel applied={echo} />);
    // The applied render is the server echo, never the editor.
    expect(screen.getByTestId("genesis-boundaries-applied")).toBeInTheDocument();
    expect(screen.queryByTestId("genesis-boundaries")).toBeNull();
    expect(screen.getByTestId("genesis-boundary-applied-class-0")).toHaveTextContent(
      "financial",
    );
    expect(screen.getByTestId("genesis-boundary-applied-class-1")).toHaveTextContent(
      "legal",
    );
    expect(
      screen.getByTestId("genesis-boundary-applied-threshold"),
    ).toHaveTextContent("500");
  });
});
