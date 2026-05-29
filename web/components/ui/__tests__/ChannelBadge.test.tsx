import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ChannelBadge } from "../ChannelBadge";
import { CHANNEL_KINDS, type ChannelKind } from "@/lib/types/channels";

describe("ChannelBadge", () => {
  it("renders the channel icon + capitalized label", () => {
    render(<ChannelBadge kind="slack" />);
    const badge = screen.getByTestId("channel-badge");
    expect(badge).toHaveAttribute("data-kind", "slack");
    expect(badge).toHaveTextContent(/slack/i);
  });

  it("accepts an explicit label override", () => {
    render(<ChannelBadge kind="voice" label="Twilio Voice" />);
    expect(screen.getByText("Twilio Voice")).toBeInTheDocument();
  });

  it("renders a discord tile that closes F12 (Discord referenced nowhere visible)", () => {
    render(<ChannelBadge kind="discord" />);
    const badge = screen.getByTestId("channel-badge");
    expect(badge).toHaveAttribute("data-kind", "discord");
    expect(badge).toHaveTextContent(/discord/i);
  });

  it("compact mode renders icon-only with channel name in title + aria-label", () => {
    render(<ChannelBadge kind="discord" compact />);
    const badge = screen.getByTestId("channel-badge");
    expect(badge).toHaveAttribute("data-compact", "true");
    expect(badge).toHaveAttribute("title", "Discord");
    expect(badge).toHaveAttribute("aria-label", "Discord");
    // Compact mode suppresses the visible text label entirely.
    expect(badge.textContent ?? "").toBe("");
  });

  it.each(CHANNEL_KINDS)(
    "renders a glyph + a11y label for channel %s",
    (kind: ChannelKind) => {
      render(<ChannelBadge kind={kind} compact />);
      const badge = screen.getByTestId("channel-badge");
      expect(badge).toHaveAttribute("data-kind", kind);
      // Every kind carries a non-empty accessible name in compact mode.
      const label = badge.getAttribute("aria-label") ?? "";
      expect(label.length).toBeGreaterThan(0);
      // And a glyph (lucide svg) for assistive-tech-aware sighted users.
      expect(badge.querySelector("svg")).not.toBeNull();
    },
  );

  // Regression: the live audit feed emits non-messaging action surfaces
  // (`agent` / `mcp` / `computer-use`) and could emit future values
  // outside ChannelKind. An unknown `kind` used to resolve to `undefined`
  // and render `<undefined />`, throwing "Element type is invalid" and
  // taking the whole /audit route down via its error boundary. The badge
  // must now render a fallback glyph + readable label instead of crashing.
  it.each([
    ["agent", "Agent"],
    ["mcp", "MCP"],
    ["computer-use", "Computer use"],
    ["totally-unknown", "Totally Unknown"],
  ])("renders a non-crashing fallback for surface %s", (kind, expectedLabel) => {
    // `kind` accepts any string (the gateway sends values outside the
    // ChannelKind union); the badge resolves a fallback instead of crashing.
    render(<ChannelBadge kind={kind} />);
    const badge = screen.getByTestId("channel-badge");
    expect(badge).toHaveAttribute("data-kind", kind);
    expect(badge.querySelector("svg")).not.toBeNull();
    expect(badge).toHaveTextContent(expectedLabel);
  });
});
