import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ChannelChip } from "./ChannelChip";
import { CHANNEL_KINDS, type ChannelKind } from "@/lib/types/channels";

describe("ChannelChip", () => {
  it("renders the channel glyph + label for a known channel", () => {
    render(<ChannelChip channel="slack" />);
    const chip = screen.getByTestId("channel-chip-slack");
    expect(chip).toHaveAttribute("data-channel", "slack");
    expect(chip).toHaveTextContent(/slack/i);
    expect(chip.querySelector("svg")).not.toBeNull();
  });

  // The gateway marketing-draft route (`routes_marketing_draft_response.rs`)
  // emits `channel: "x" | "linkedin"` on a queued Marcus draft. Before these
  // were in the ChannelKind union, the icon map returned `undefined` →
  // `<undefined />` → "Element type is invalid" crashed the whole /approvals
  // inbox via its route error boundary. They must now render real chips.
  it.each([
    ["x", "X"],
    ["linkedin", "LinkedIn"],
  ])("renders the publishing-surface chip for channel %s", (channel, label) => {
    render(<ChannelChip channel={channel as ChannelKind} />);
    const chip = screen.getByTestId(`channel-chip-${channel}`);
    expect(chip).toHaveAttribute("data-channel", channel);
    expect(chip).toHaveTextContent(label);
    expect(chip.querySelector("svg")).not.toBeNull();
  });

  it.each(CHANNEL_KINDS)(
    "renders a glyph + non-empty label for every known channel %s",
    (channel: ChannelKind) => {
      render(<ChannelChip channel={channel} />);
      const chip = screen.getByTestId(`channel-chip-${channel}`);
      expect(chip).toHaveAttribute("data-channel", channel);
      expect((chip.textContent ?? "").trim().length).toBeGreaterThan(0);
      expect(chip.querySelector("svg")).not.toBeNull();
    },
  );

  // Defense-in-depth: a channel value outside the union (a future gateway
  // channel, a typo) must render a fallback chip — never `<undefined />`,
  // which would crash the inbox. The chip resolves a default icon (Globe)
  // and a humanized label instead.
  it.each([
    ["instagram", "Instagram"],
    ["telegram", "Telegram"],
    ["totally-unknown", "Totally Unknown"],
  ])("renders a non-crashing fallback for unknown channel %s", (channel, label) => {
    // `channel` is widened to string so off-union gateway values typecheck.
    render(<ChannelChip channel={channel} />);
    const chip = screen.getByTestId(`channel-chip-${channel}`);
    expect(chip).toHaveAttribute("data-channel", channel);
    expect(chip.querySelector("svg")).not.toBeNull();
    expect(chip).toHaveTextContent(label);
  });
});
