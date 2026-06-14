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
      screen.getByRole("heading", { level: 1, name: /hire an AI teammate/i }),
    ).toBeInTheDocument();
    expect(document.body).not.toHaveTextContent(/multi-provider routing/i);
    expect(document.body).not.toHaveTextContent(/provider routing/i);
  });

  it("leads with the trust pillars (the moat), reframed honestly per the 2026-06-15 re-verification", () => {
    render(<MarketingLandingPage />);

    expect(
      screen.getByRole("heading", { name: /trust layer.*let an AI act/i }),
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

  it("offers a sales-led 'Book a governance walkthrough' path (hero CTA + qualifying form)", () => {
    render(<MarketingLandingPage />);

    // The hero leads with the sales-led CTA, anchored to the booking section.
    const heroCta = screen.getByRole("link", {
      name: /book a governance walkthrough/i,
    });
    expect(heroCta).toHaveAttribute("href", "#book-walkthrough");
    // The booking section + its qualifying form (reusing the enterprise-inquiry
    // endpoint) are present; no pre-faked success/scheduler before any submit.
    expect(screen.getByTestId("walkthrough-form")).toBeInTheDocument();
    expect(screen.queryByTestId("walkthrough-success")).toBeNull();
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

  it("points the demo slot at the live demo workspace (no dead play button, no fake video)", () => {
    render(<MarketingLandingPage />);

    // While no MP4 is configured the slot is a REAL link into the live demo
    // journey — not a play button with a dead handler.
    const slot = screen.getByTestId("demo-video-slot");
    expect(slot).toBeInTheDocument();
    expect(slot.tagName).toBe("A");
    expect(slot).toHaveAttribute("href", "/auth/signup");
    expect(screen.getByTestId("demo-live-cta-pill")).toHaveTextContent(
      /try it live.*no email/i,
    );
    // NO THEATER: no <video> element and no fake "recording" pill.
    expect(document.querySelector("video")).toBeNull();
    expect(screen.queryByTestId("demo-pending-pill")).not.toBeInTheDocument();
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
