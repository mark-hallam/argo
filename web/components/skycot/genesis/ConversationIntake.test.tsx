import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

// Mock the turn endpoint BEFORE importing the component so the real fetcher
// never fires. Keep `EMPTY_CAPTURED` (and any other real exports) intact.
vi.mock("@/lib/api/genesis-conversation", async () => {
  const actual = await vi.importActual<
    typeof import("@/lib/api/genesis-conversation")
  >("@/lib/api/genesis-conversation");
  return { ...actual, postConversationTurn: vi.fn() };
});

import {
  ConversationIntake,
  gatherSlotsFromCaptured,
} from "./ConversationIntake";
import {
  EMPTY_CAPTURED,
  postConversationTurn,
  type ConversationCaptured,
  type ConversationResponse,
} from "@/lib/api/genesis-conversation";

const mockedTurn = vi.mocked(postConversationTurn);

function res(over: Partial<ConversationResponse>): ConversationResponse {
  return {
    conversation_id: "conv-test",
    status: "asking",
    captured: { ...EMPTY_CAPTURED },
    transcript: [],
    ...over,
  };
}

const FILLED: ConversationCaptured = {
  charter: "Keep the dispatch board clear every morning.",
  persona: "Calm, concise, proactive.",
  responsibilities: ["Triage inbound tickets", "Escalate blockers"],
  cadence: "every weekday at 9am",
  exceptions: ["never touch legal escalations"],
  success_criteria: [
    { name: "on-time dispatch", kind: "percentage", target: 95, comparator: "gte" },
  ],
};

describe("ConversationIntake", () => {
  beforeEach(() => {
    mockedTurn.mockReset();
  });

  it("fires the opening turn and renders the assistant's first message", async () => {
    mockedTurn.mockResolvedValueOnce(
      res({
        status: "asking",
        assistant_message: "Hi! What should this teammate own?",
        transcript: [
          { speaker: "assistant", text: "Hi! What should this teammate own?" },
        ],
      }),
    );
    render(<ConversationIntake tenantId="op-1" onComplete={vi.fn()} />);
    await waitFor(() =>
      expect(screen.getByTestId("genesis-conversation-bubble-0")).toHaveTextContent(
        "What should this teammate own?",
      ),
    );
    // Opening turn sends NO message.
    expect(mockedTurn).toHaveBeenCalledWith(
      expect.objectContaining({ tenantId: "op-1", message: undefined }),
    );
  });

  it("drives a back-and-forth and surfaces the captured slots live", async () => {
    mockedTurn
      .mockResolvedValueOnce(
        res({
          status: "asking",
          transcript: [{ speaker: "assistant", text: "What's the role?" }],
        }),
      )
      .mockResolvedValueOnce(
        res({
          status: "ready",
          captured: FILLED,
          transcript: [
            { speaker: "assistant", text: "What's the role?" },
            { speaker: "operator", text: "A dispatch coordinator" },
            { speaker: "assistant", text: "Great — ready when you are." },
          ],
        }),
      );
    render(<ConversationIntake tenantId="op-1" onComplete={vi.fn()} />);
    await waitFor(() =>
      expect(screen.getByTestId("genesis-conversation-input")).toBeInTheDocument(),
    );

    fireEvent.change(screen.getByTestId("genesis-conversation-input"), {
      target: { value: "A dispatch coordinator" },
    });
    fireEvent.click(screen.getByTestId("genesis-conversation-send"));

    // Captured panel reflects the filled slots (charter + reused cadence input).
    await waitFor(() =>
      expect(
        screen.getByTestId("genesis-captured-charter"),
      ).toHaveTextContent("dispatch board"),
    );
    expect(screen.getByTestId("genesis-conversation-ready")).toBeInTheDocument();
    // The reused CadenceExceptionsCapture slot rendering shows the cadence.
    expect(screen.getByTestId("genesis-cadence-input")).toHaveValue(
      "every weekday at 9am",
    );
    // Second turn forwarded the operator's message.
    expect(mockedTurn).toHaveBeenLastCalledWith(
      expect.objectContaining({ message: "A dispatch coordinator" }),
    );
  });

  it("hands the gathered slots up on confirm (status: done)", async () => {
    const onComplete = vi.fn();
    mockedTurn
      .mockResolvedValueOnce(
        res({ status: "ready", captured: FILLED, transcript: [
          { speaker: "assistant", text: "Ready." },
        ] }),
      )
      .mockResolvedValueOnce(res({ status: "done", captured: FILLED }));
    render(<ConversationIntake tenantId="op-1" onComplete={onComplete} />);

    await waitFor(() =>
      expect(screen.getByTestId("genesis-conversation-confirm")).toBeInTheDocument(),
    );
    fireEvent.click(screen.getByTestId("genesis-conversation-confirm"));

    await waitFor(() => expect(onComplete).toHaveBeenCalledTimes(1));
    expect(mockedTurn).toHaveBeenLastCalledWith(
      expect.objectContaining({ confirm: true }),
    );
    const gathered = onComplete.mock.calls[0][0];
    expect(gathered.roleDescription).toContain("dispatch board");
    expect(gathered.cadence).toBe("every weekday at 9am");
    expect(gathered.exceptions).toEqual(["never touch legal escalations"]);
    expect(gathered.successCriteria).toHaveLength(1);
  });

  it("surfaces an error when a turn fails", async () => {
    mockedTurn.mockRejectedValueOnce(new Error("gateway down"));
    render(<ConversationIntake tenantId="op-1" onComplete={vi.fn()} />);
    await waitFor(() =>
      expect(screen.getByTestId("genesis-conversation-error")).toHaveTextContent(
        "gateway down",
      ),
    );
  });
});

describe("gatherSlotsFromCaptured", () => {
  it("composes a role description from charter, responsibilities, and persona", () => {
    const g = gatherSlotsFromCaptured(FILLED);
    expect(g.roleDescription).toContain("dispatch board");
    expect(g.roleDescription).toContain("Triage inbound tickets");
    expect(g.roleDescription).toContain("Persona:");
    expect(g.cadence).toBe("every weekday at 9am");
    expect(g.successCriteria).toEqual(FILLED.success_criteria);
  });

  it("yields an empty description from an empty captured block", () => {
    const g = gatherSlotsFromCaptured(EMPTY_CAPTURED);
    expect(g.roleDescription).toBe("");
    expect(g.exceptions).toEqual([]);
  });
});
