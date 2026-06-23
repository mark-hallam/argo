import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { GenesisPreviewEmbed } from "../GenesisPreviewEmbed";

const push = vi.fn();
vi.mock("next/navigation", () => ({ useRouter: () => ({ push }) }));

describe("GenesisPreviewEmbed", () => {
  beforeEach(() => push.mockClear());

  it("renders a describe-the-job input + a real-shaped teammate spec + the start-free CTA", () => {
    render(<GenesisPreviewEmbed />);
    expect(screen.getByTestId("genesis-input")).toBeInTheDocument();
    expect(screen.getByTestId("genesis-spec")).toBeInTheDocument();
    expect(screen.getByTestId("genesis-start-free")).toBeInTheDocument();
    // NO-THEATER: the shown spec is labeled an example, generated live on signup.
    expect(screen.getByTestId("genesis-sample-note")).toHaveTextContent(
      /example of a real genesis output/i,
    );
  });

  it("example chips switch the displayed teammate spec", () => {
    render(<GenesisPreviewEmbed />);
    const spec = screen.getByTestId("genesis-spec");
    expect(spec).toHaveTextContent("Priya"); // first example seeded
    fireEvent.click(screen.getByTestId("genesis-chip-1"));
    expect(spec).toHaveTextContent("Jamal"); // second example after click
  });

  it("routes 'describe your own' into self-serve signup — NO live LLM call (no cost-DoS)", () => {
    render(<GenesisPreviewEmbed />);
    fireEvent.change(screen.getByTestId("genesis-input"), {
      target: { value: "chase overdue invoices" },
    });
    fireEvent.click(screen.getByTestId("genesis-generate"));
    expect(push).toHaveBeenCalledWith(
      expect.stringContaining("/auth/signup?role=chase%20overdue%20invoices"),
    );
  });
});
