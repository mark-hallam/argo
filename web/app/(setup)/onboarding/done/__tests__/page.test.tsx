/**
 * Acceptance test for
 * `fix-onboarding-add-done-page-with-meet-your-teammate-handoff`.
 *
 * Pins:
 *  - The page renders the locked F1 editorial chrome (eyebrow + hero +
 *    Meet/View-team CTAs).
 *  - The worker name passed in `?name=` lands in the hero accent slot.
 *  - The primary CTA + 6s auto-advance both `router.replace` to
 *    `/workers/<id>`.
 *  - Clicking the primary CTA cancels the timeout.
 *  - The `sessionStorage` breadcrumb prevents re-firing the celebration
 *    on a same-tab re-visit.
 */

import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const routerMock = vi.hoisted(() => ({
  push: vi.fn(),
  replace: vi.fn(),
  back: vi.fn(),
}));

const searchParamsMock = vi.hoisted(() => {
  const state = new URLSearchParams();
  return {
    state,
    get: vi.fn((key: string) => state.get(key)),
    set(query: string) {
      Array.from(state.keys()).forEach((k) => state.delete(k));
      const next = new URLSearchParams(query);
      next.forEach((v, k) => state.set(k, v));
    },
  };
});

vi.mock("next/navigation", () => ({
  useRouter: () => routerMock,
  useSearchParams: () => ({ get: (k: string) => searchParamsMock.get(k) }),
  usePathname: () => "/onboarding/done",
}));

import OnboardingDonePage from "../page";

const ADVANCE_MS = 6000;

describe("/onboarding/done celebration page", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    routerMock.push.mockReset();
    routerMock.replace.mockReset();
    routerMock.back.mockReset();
    searchParamsMock.get.mockClear();
    if (typeof window !== "undefined") {
      window.sessionStorage.clear();
    }
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders the worker name in the hero gradient slot", () => {
    searchParamsMock.set("worker=worker-123&name=Maya");

    render(<OnboardingDonePage />);

    const hero = screen.getByTestId("wiz-shell-hero");
    expect(hero.textContent).toContain("Maya");
    expect(hero.textContent).toContain("is ready.");
    expect(screen.getByTestId("wiz-shell-eyebrow")).toHaveTextContent(
      "MEET YOUR FIRST TEAMMATE",
    );
  });

  it("auto-advances to /workers/<id> after 6 seconds when the user does nothing", () => {
    searchParamsMock.set("worker=worker-123&name=Maya");

    render(<OnboardingDonePage />);

    expect(routerMock.replace).not.toHaveBeenCalled();
    act(() => {
      vi.advanceTimersByTime(ADVANCE_MS);
    });
    expect(routerMock.replace).toHaveBeenCalledWith("/workers/worker-123");
  });

  it("cancels the timeout when the user clicks the primary CTA", () => {
    searchParamsMock.set("worker=worker-123&name=Maya");

    render(<OnboardingDonePage />);

    fireEvent.click(screen.getByTestId("onboarding-done-meet-cta"));
    expect(routerMock.replace).toHaveBeenCalledTimes(1);
    expect(routerMock.replace).toHaveBeenCalledWith("/workers/worker-123");
    expect(screen.getByTestId("onboarding-done-auto-advance")).toHaveAttribute(
      "data-state",
      "cancelled",
    );

    act(() => {
      vi.advanceTimersByTime(ADVANCE_MS * 2);
    });
    // Should not have fired a second navigation from the (now-cleared) timer.
    expect(routerMock.replace).toHaveBeenCalledTimes(1);
  });

  it("routes the secondary CTA to /team", () => {
    searchParamsMock.set("worker=worker-123&name=Maya");

    render(<OnboardingDonePage />);

    fireEvent.click(screen.getByTestId("onboarding-done-view-team"));
    expect(routerMock.push).toHaveBeenCalledWith("/team");
  });

  it("skips the celebration if the user re-visits the URL in the same tab", () => {
    searchParamsMock.set("worker=worker-123&name=Maya");
    window.sessionStorage.setItem(
      "skycot:onboarding-done-seen:worker-123",
      "1",
    );

    render(<OnboardingDonePage />);

    expect(routerMock.replace).toHaveBeenCalledWith("/workers/worker-123");
  });

  it("falls back to a generic name and /team when no worker id is supplied", () => {
    searchParamsMock.set("");

    render(<OnboardingDonePage />);

    expect(screen.getByTestId("wiz-shell-hero").textContent).toContain(
      "Your first teammate",
    );
    act(() => {
      vi.advanceTimersByTime(ADVANCE_MS);
    });
    expect(routerMock.replace).toHaveBeenCalledWith("/team");
  });
});
