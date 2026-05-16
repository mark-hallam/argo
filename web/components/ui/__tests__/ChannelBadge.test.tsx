import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ChannelBadge } from "../ChannelBadge";

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
});
