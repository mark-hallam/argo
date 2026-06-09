import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import MarketingLandingPage from "./page";

vi.mock("next/navigation", () => ({
  usePathname: () => "/landing",
}));

describe("MarketingLandingPage", () => {
  it("does not market multi-provider routing as a v1 hero capability", () => {
    render(<MarketingLandingPage />);

    expect(
      screen.getByRole("heading", { level: 1, name: /hire a teammate/i }),
    ).toBeInTheDocument();
    expect(document.body).not.toHaveTextContent(/multi-provider routing/i);
    expect(document.body).not.toHaveTextContent(/provider routing/i);
  });

  it("leads with the four verified-unique differentiators (the moat)", () => {
    render(<MarketingLandingPage />);

    expect(
      screen.getByRole("heading", { name: /four things.*ships/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /Every action, provable/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /knows what it's not allowed to say/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /Nothing risky without your yes/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /Hire teammates, not chatbots/i }),
    ).toBeInTheDocument();
  });

  it("renders a working waitlist capture (hero + final) and does not pre-fake success", () => {
    render(<MarketingLandingPage />);

    const forms = screen.getAllByTestId("waitlist-form");
    expect(forms.length).toBeGreaterThanOrEqual(2);
    // The submit affordance exists and reads as a request, not a fake confirmation.
    expect(screen.getAllByTestId("waitlist-submit")[0]).toHaveTextContent(
      /request early access/i,
    );
    // No success banner is shown before any submit.
    expect(screen.queryByTestId("waitlist-success")).not.toBeInTheDocument();
  });

  it("shows a labeled demo-video placeholder (not a fake video)", () => {
    render(<MarketingLandingPage />);

    const slot = screen.getByTestId("demo-video-slot");
    expect(slot).toBeInTheDocument();
    expect(screen.getByTestId("demo-pending-pill")).toHaveTextContent(/recording/i);
    // No <video> element is rendered while the MP4 does not exist.
    expect(document.querySelector("video")).toBeNull();
  });

  it("renders the Portia embed as a LIVE, truth-bounded widget with an ask input", () => {
    render(<MarketingLandingPage />);

    const embed = screen.getByTestId("portia-embed");
    // The widget is now wired to the live public route — labeled LIVE, not a
    // canned preview. The proof of the truth-boundary moat is real.
    expect(within(embed).getByTestId("portia-live-pill")).toHaveTextContent(/live/i);
    // A free-text ask affordance is present (the visitor's own question goes
    // to POST /api/v1/pitch/ask/public).
    expect(within(embed).getByTestId("portia-input")).toBeInTheDocument();
    expect(within(embed).getByTestId("portia-send")).toBeInTheDocument();
    // Honest safety note: truth-bounded live answering, with a clearly-labeled
    // sample only when the live service is unavailable (NO-THEATER).
    expect(screen.getByTestId("portia-safety-note")).toHaveTextContent(/truth-bounded/i);
  });
});
