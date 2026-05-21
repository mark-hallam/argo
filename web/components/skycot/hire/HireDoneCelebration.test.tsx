import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HireDoneCelebration } from "./HireDoneCelebration";

/**
 * Surface tests for the /hire/done viral-moment payoff. Verifies:
 * - BrandLoudHero copy uses the just-hired worker's name.
 * - Welcome card avatar derives initials and renders the role.
 * - "Say hi in Slack" and "View profile" CTAs are wired correctly.
 * - Confetti burst respects `prefers-reduced-motion`.
 */

type MatchMedia = (q: string) => MediaQueryList;
let originalMatchMedia: MatchMedia | undefined;

function mockMatchMedia(matches: boolean): void {
  const mql: MediaQueryList = {
    matches,
    media: "(prefers-reduced-motion: reduce)",
    onchange: null,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
    addListener: () => undefined,
    removeListener: () => undefined,
    dispatchEvent: () => false,
  } as unknown as MediaQueryList;
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: vi.fn().mockReturnValue(mql),
  });
}

beforeEach(() => {
  originalMatchMedia = window.matchMedia as unknown as MatchMedia | undefined;
  mockMatchMedia(false);
});

afterEach(() => {
  if (originalMatchMedia) {
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      writable: true,
      value: originalMatchMedia,
    });
  } else {
    // happy-dom doesn't ship matchMedia by default; drop the mock so
    // it doesn't leak into the next test.
    Reflect.deleteProperty(window, "matchMedia");
  }
});

describe("HireDoneCelebration", () => {
  it("renders the BrandLoudHero with `Meet <name>.` and the welcome card", () => {
    render(
      <HireDoneCelebration
        workerId="wkr_abc123"
        workerName="Mariah"
        role="Receptionist · Front desk"
      />,
    );

    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(
      "Meet Mariah.",
    );
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "Mariah",
    );
    expect(screen.getByText("Receptionist · Front desk")).toBeInTheDocument();
  });

  it("derives initials from a multi-word name", () => {
    render(
      <HireDoneCelebration
        workerId="wkr_abc123"
        workerName="Mariah Sinclair"
      />,
    );
    const card = screen.getByTestId("hire-done-welcome-card");
    expect(card.textContent).toContain("MS");
  });

  it("falls back to a 2-letter initial for a single name", () => {
    render(<HireDoneCelebration workerId="wkr_x" workerName="Patrick" />);
    const card = screen.getByTestId("hire-done-welcome-card");
    expect(card.textContent).toContain("PA");
  });

  it("wires the CTAs: View profile → /workers/<id>/overview, Slack → slackHref", () => {
    render(
      <HireDoneCelebration
        workerId="wkr_abc123"
        workerName="Mariah"
        slackHref="https://slack.com/skycot-demo"
      />,
    );
    expect(screen.getByTestId("hire-done-slack-cta")).toHaveAttribute(
      "href",
      "https://slack.com/skycot-demo",
    );
    expect(screen.getByTestId("hire-done-profile-cta")).toHaveAttribute(
      "href",
      "/workers/wkr_abc123/overview",
    );
  });

  it("renders confetti when motion is allowed", async () => {
    mockMatchMedia(false);
    render(<HireDoneCelebration workerId="wkr_x" workerName="Anna" />);
    expect(await screen.findByTestId("hire-done-confetti")).toBeInTheDocument();
  });

  it("skips the confetti burst when `prefers-reduced-motion: reduce` is set", () => {
    mockMatchMedia(true);
    render(<HireDoneCelebration workerId="wkr_x" workerName="Anna" />);
    expect(screen.queryByTestId("hire-done-confetti")).not.toBeInTheDocument();
  });

  it("hides the role row when no role is provided", () => {
    render(<HireDoneCelebration workerId="wkr_x" workerName="Anna" />);
    const card = screen.getByTestId("hire-done-welcome-card");
    // Only the heading should carry the name; no descriptor row.
    const paragraphs = card.querySelectorAll("p.skycot-hire-done-role");
    expect(paragraphs.length).toBe(0);
  });
});
