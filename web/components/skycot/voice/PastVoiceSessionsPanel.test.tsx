import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  PastVoiceSessionsPanel,
  relativeTime,
} from "@/components/skycot/voice/PastVoiceSessionsPanel";
import { listVoiceSessions } from "@/lib/api/voice";

vi.mock("@/lib/api/voice", () => ({
  listVoiceSessions: vi.fn(),
}));

const mockList = vi.mocked(listVoiceSessions);

afterEach(() => {
  vi.clearAllMocks();
});

describe("relativeTime", () => {
  it("formats recent timestamps and guards empty input", () => {
    const now = Date.now();
    expect(relativeTime(now - 5 * 60_000)).toBe("5 min ago");
    expect(relativeTime(now - 3 * 60 * 60_000)).toBe("3 h ago");
    expect(relativeTime(0)).toBe("");
  });
});

describe("PastVoiceSessionsPanel", () => {
  it("filters by mode and shows the audit-framed empty state", async () => {
    mockList.mockResolvedValue([]);
    render(<PastVoiceSessionsPanel mode="pitch" agentName="Portia" />);
    expect(
      await screen.findByText(/Every live call with Portia is transcribed/i),
    ).toBeInTheDocument();
    expect(mockList).toHaveBeenCalledWith("pitch");
  });

  it("lists a session and expands to the full both-sides transcript", async () => {
    const now = Date.now();
    mockList.mockResolvedValue([
      {
        session_id: "s1",
        mode: "pitch",
        started_at: now - 60_000,
        ended_at: now - 30_000,
        turn_count: 2,
        preview: "what is our strongest moat",
        turns: [
          { role: "user", text: "what is our strongest moat", ts: now - 60_000 },
          { role: "agent", text: "the hash-chained tamper-evident audit", ts: now - 30_000 },
        ],
      },
    ]);
    render(<PastVoiceSessionsPanel mode="pitch" agentName="Portia" />);

    const summary = await screen.findByRole("button", { name: /strongest moat/i });
    // Turns stay hidden until the row is expanded.
    expect(screen.queryByText(/hash-chained tamper-evident audit/i)).not.toBeInTheDocument();

    fireEvent.click(summary);

    // Expanded: both sides render, labelled "You" and the agent's name.
    expect(screen.getByText(/hash-chained tamper-evident audit/i)).toBeInTheDocument();
    expect(screen.getByText("You")).toBeInTheDocument();
    expect(screen.getByText("Portia")).toBeInTheDocument();
  });
});
