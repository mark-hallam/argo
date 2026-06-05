import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { ConversationCapturedPanel } from "./ConversationCapturedPanel";
import { EMPTY_CAPTURED } from "@/lib/api/genesis-conversation";

describe("ConversationCapturedPanel", () => {
  it("shows the empty prompt before any slot is filled", () => {
    render(<ConversationCapturedPanel captured={EMPTY_CAPTURED} />);
    expect(screen.getByTestId("genesis-captured-empty")).toBeInTheDocument();
    expect(
      screen.queryByTestId("genesis-captured-charter"),
    ).not.toBeInTheDocument();
  });

  it("renders the free-text slots as they fill", () => {
    render(
      <ConversationCapturedPanel
        captured={{
          ...EMPTY_CAPTURED,
          charter: "Own the morning dispatch board.",
          persona: "Calm and concise.",
          responsibilities: ["Triage tickets", "Escalate blockers"],
        }}
      />,
    );
    expect(screen.queryByTestId("genesis-captured-empty")).not.toBeInTheDocument();
    expect(screen.getByTestId("genesis-captured-charter")).toHaveTextContent(
      "dispatch board",
    );
    expect(screen.getByTestId("genesis-captured-persona")).toHaveTextContent(
      "Calm and concise",
    );
    expect(
      screen.getByTestId("genesis-captured-responsibilities"),
    ).toHaveTextContent("Escalate blockers");
  });

  it("reuses the CadenceExceptionsCapture slot rendering for typed slots", () => {
    render(
      <ConversationCapturedPanel
        captured={{
          ...EMPTY_CAPTURED,
          cadence: "every weekday at 9am",
          exceptions: ["never touch legal escalations"],
        }}
      />,
    );
    // The reused (disabled) capture panel renders.
    expect(
      screen.getByTestId("genesis-cadence-exceptions"),
    ).toBeInTheDocument();
    const cadence = screen.getByTestId("genesis-cadence-input");
    expect(cadence).toHaveValue("every weekday at 9am");
    expect(cadence).toBeDisabled();
  });
});
