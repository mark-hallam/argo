import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

/**
 * `/genesis` page-level regression for the "Talk it through" toggle
 * (UF Phase-A · A3, DEC-039 / DEC-048 rule 1). Pins that:
 *  - the toggle appears,
 *  - "Talk it through" (the guided interview) is now the DEFAULT (DEC-048),
 *  - switching to "Describe it" swaps in the one-shot surface,
 *  - switching back restores the conversation surface.
 */

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn() }),
}));

// Keep `GENESIS_PHASES` + types real; stub only the network calls.
vi.mock("@/lib/api/genesis", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/api/genesis")>("@/lib/api/genesis");
  return { ...actual, streamGenesis: vi.fn(), hireFromGenesis: vi.fn() };
});

vi.mock("@/lib/api/genesis-conversation", async () => {
  const actual = await vi.importActual<
    typeof import("@/lib/api/genesis-conversation")
  >("@/lib/api/genesis-conversation");
  return { ...actual, postConversationTurn: vi.fn() };
});

import GenesisPage from "@/app/(app)/genesis/page";
import { postConversationTurn, EMPTY_CAPTURED } from "@/lib/api/genesis-conversation";

const mockedTurn = vi.mocked(postConversationTurn);

describe("/genesis Talk-it-through toggle", () => {
  beforeEach(() => {
    mockedTurn.mockReset();
    mockedTurn.mockResolvedValue({
      conversation_id: "conv-test",
      status: "asking",
      assistant_message: "What should this teammate own?",
      captured: { ...EMPTY_CAPTURED },
      transcript: [
        { speaker: "assistant", text: "What should this teammate own?" },
      ],
    });
  });

  it("defaults to the guided 'Talk it through' interview (DEC-048)", async () => {
    render(<GenesisPage />);
    expect(screen.getByTestId("genesis-mode-toggle")).toBeInTheDocument();
    expect(screen.getByTestId("genesis-mode-talk")).toHaveAttribute(
      "aria-selected",
      "true",
    );
    // Conversation surface present by default; one-shot describe absent.
    await waitFor(() =>
      expect(screen.getByTestId("genesis-conversation")).toBeInTheDocument(),
    );
    expect(screen.queryByTestId("genesis-hero-form")).not.toBeInTheDocument();
  });

  it("swaps in the one-shot 'Describe it' surface and back", async () => {
    render(<GenesisPage />);
    // Starts in talk mode (default).
    await waitFor(() =>
      expect(screen.getByTestId("genesis-conversation")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByTestId("genesis-mode-describe"));
    // The original one-shot flow is one click away, intact.
    expect(screen.getByTestId("genesis-hero-form")).toBeInTheDocument();
    expect(screen.getByTestId("genesis-cadence-exceptions")).toBeInTheDocument();
    expect(screen.queryByTestId("genesis-conversation")).not.toBeInTheDocument();

    // Toggle back — the guided interview returns.
    fireEvent.click(screen.getByTestId("genesis-mode-talk"));
    await waitFor(() =>
      expect(screen.getByTestId("genesis-conversation")).toBeInTheDocument(),
    );
    expect(screen.queryByTestId("genesis-hero-form")).not.toBeInTheDocument();
  });
});
