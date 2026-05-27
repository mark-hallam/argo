/**
 * Acceptance test for
 * `fix-onboarding-welcome-headline-singular-teammate-vs-workforce`.
 *
 * Pins the singular hire-a-teammate framing on /onboarding/welcome and
 * the demotion of the Genesis pillar from a clickable deep-link into a
 * passive marketing affordance, so the 5-step wizard narrative stays
 * intact on camera. A regular "Try Genesis studio" link below the CTA
 * preserves discoverability without competing for the primary click.
 */

import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

const routerMock = vi.hoisted(() => ({
  push: vi.fn(),
  replace: vi.fn(),
  back: vi.fn(),
}));

const draftMock = vi.hoisted(() => {
  const state = {
    setStep: vi.fn(),
  };
  type Selector = (s: typeof state) => unknown;
  const hook = vi.fn((selector: Selector) => selector(state));
  return { state, hook };
});

vi.mock("next/navigation", () => ({
  useRouter: () => routerMock,
}));

vi.mock("@/lib/onboarding/use-onboarding-draft", () => ({
  useOnboardingDraft: draftMock.hook,
}));

import OnboardingWelcomePage from "../page";

describe("onboarding welcome — singular teammate framing", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders the singular hire-a-teammate headline with the gradient on 'first teammate'", () => {
    render(<OnboardingWelcomePage />);

    const hero = screen.getByTestId("wiz-shell-hero");
    expect(hero.textContent ?? "").toMatch(/Hire your first teammate\./);
    // Must not regress to fleet/tool plural framing.
    expect(hero.textContent ?? "").not.toMatch(/AI workforce/i);

    // Gradient (accent) span carries the "first teammate" phrase.
    const accent = hero.querySelector(".wiz-hero-accent");
    expect(accent).not.toBeNull();
    expect(accent?.textContent ?? "").toMatch(/first teammate/);
  });

  it("renders the Genesis pillar as a non-interactive marketing affordance", () => {
    render(<OnboardingWelcomePage />);

    const pillar = screen.getByTestId("onboarding-pillar-genesis-studio");
    expect(pillar.getAttribute("role")).toBeNull();
    expect(pillar.getAttribute("tabindex")).toBeNull();
    expect(pillar.getAttribute("onclick")).toBeNull();
  });

  it("does not navigate to /genesis when the Genesis pillar is clicked", () => {
    render(<OnboardingWelcomePage />);

    const pillar = screen.getByTestId("onboarding-pillar-genesis-studio");
    pillar.dispatchEvent(new MouseEvent("click", { bubbles: true }));

    expect(routerMock.push).not.toHaveBeenCalledWith("/genesis");
  });

  it("surfaces Genesis as an explicit aside link below the primary CTA with a return param", () => {
    render(<OnboardingWelcomePage />);

    const link = screen.getByTestId("onboarding-genesis-link");
    expect(link.getAttribute("href")).toBe(
      "/genesis?return=/onboarding/welcome",
    );
  });
});
