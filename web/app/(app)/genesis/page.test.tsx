import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

/**
 * `/genesis` page-level regression for the additive "Talk it through" toggle
 * (UF Phase-A · A3, DEC-039). Pins that:
 *  - the toggle appears,
 *  - "Describe it" (the original one-shot flow) is the default and unchanged,
 *  - switching to "Talk it through" swaps in the conversation surface,
 *  - switching back restores the one-shot describe surface.
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

  it("defaults to the unchanged one-shot 'Describe it' flow", () => {
    render(<GenesisPage />);
    expect(screen.getByTestId("genesis-mode-toggle")).toBeInTheDocument();
    expect(screen.getByTestId("genesis-mode-describe")).toHaveAttribute(
      "aria-selected",
      "true",
    );
    // One-shot surface present; conversation absent.
    expect(screen.getByTestId("genesis-hero-form")).toBeInTheDocument();
    expect(screen.getByTestId("genesis-cadence-exceptions")).toBeInTheDocument();
    expect(screen.queryByTestId("genesis-conversation")).not.toBeInTheDocument();
    expect(mockedTurn).not.toHaveBeenCalled();
  });

  it("swaps in the conversation surface on 'Talk it through' and back", async () => {
    render(<GenesisPage />);
    fireEvent.click(screen.getByTestId("genesis-mode-talk"));

    await waitFor(() =>
      expect(screen.getByTestId("genesis-conversation")).toBeInTheDocument(),
    );
    // The one-shot describe surface is hidden in talk mode.
    expect(screen.queryByTestId("genesis-hero-form")).not.toBeInTheDocument();
    expect(mockedTurn).toHaveBeenCalledTimes(1);

    // Toggle back — the original flow returns intact.
    fireEvent.click(screen.getByTestId("genesis-mode-describe"));
    expect(screen.getByTestId("genesis-hero-form")).toBeInTheDocument();
    expect(screen.queryByTestId("genesis-conversation")).not.toBeInTheDocument();
  });
});
