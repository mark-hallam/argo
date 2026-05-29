/**
 * Acceptance test for
 * `rebase-fix-onboarding-provider-key-demo-mode-mask-and-skip-affordance`
 * (originally `fix-onboarding-provider-key-demo-mode-mask-and-skip-affordance`).
 *
 * Pins the demoMode placeholder swap on /onboarding/connect-provider so
 * that the default `sk-ant-...` shape-hint placeholder — which reads as
 * a real-key invitation — is replaced in demoMode with a placeholder
 * that signals "paste anything works" up-front, BEFORE the user has
 * typed (or pasted) what could be a real key. Closes UX-AUDIT-T0739MEL
 * F11 (P2).
 *
 * The skip-button promotion and above-input amber banner from the
 * original brief were dropped per the rebase brief's guidance:
 *   - Skip promotion would invert the CTA hierarchy pinned by #2578
 *     (cta-hierarchy.test.tsx).
 *   - The above-input banner would collide with the global shell
 *     demo-banner dedupe from #2551; the existing demoMode sub-text
 *     (page.tsx:199-203) already serves the "tell the buyer it's
 *     mocked" job.
 */

import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const routerMock = vi.hoisted(() => ({
  push: vi.fn(),
  replace: vi.fn(),
  back: vi.fn(),
}));

const draftMock = vi.hoisted(() => {
  const state = {
    // #2660 added a `hasHydrated` gate to the page: it renders the
    // WizardSkeleton until the persisted draft store rehydrates. The
    // mock must report hydrated so the form (and its provider-key
    // input) renders under test.
    hasHydrated: true,
    setStep: vi.fn(),
    setProvider: vi.fn(),
  };
  type Selector = (s: typeof state) => unknown;
  const hook = vi.fn((selector: Selector) => selector(state));
  return { state, hook };
});

const providerKeyMock = vi.hoisted(() => ({
  mutateAsync: vi.fn(),
  isPending: false,
}));

const demoModeMock = vi.hoisted(() => ({
  isDemoMode: vi.fn((): boolean => false),
}));

vi.mock("next/navigation", () => ({
  useRouter: () => routerMock,
  usePathname: () => "/onboarding/connect-provider",
}));

vi.mock("@/lib/onboarding/use-onboarding-draft", () => ({
  useOnboardingDraft: draftMock.hook,
}));

vi.mock("@/lib/onboarding/use-provider-key", () => ({
  useProviderKey: () => providerKeyMock,
}));

vi.mock("@/lib/skycot/demo-mode", () => demoModeMock);

const apiClientMock = vi.hoisted(() => {
  class TestApiError extends Error {
    status: number;
    code?: string;
    body?: unknown;
    constructor(status: number, message: string, code?: string) {
      super(message);
      this.status = status;
      this.code = code;
    }
  }
  return { ApiError: TestApiError, apiPost: vi.fn() };
});

vi.mock("@/lib/api/client", () => apiClientMock);

import OnboardingConnectProviderPage from "../page";

describe("onboarding connect-provider — demoMode placeholder", () => {
  beforeEach(() => {
    providerKeyMock.mutateAsync.mockReset();
    providerKeyMock.isPending = false;
    demoModeMock.isDemoMode.mockReturnValue(false);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("uses the honest 'anything-works' placeholder when isDemoMode() is true", () => {
    demoModeMock.isDemoMode.mockReturnValue(true);

    render(<OnboardingConnectProviderPage />);

    const input = screen.getByTestId("provider-key") as HTMLInputElement;
    expect(input.placeholder).toBe("sk-ant-anything-works-here-in-demo");
  });

  it("keeps the shape-hint placeholder when isDemoMode() is false", () => {
    demoModeMock.isDemoMode.mockReturnValue(false);

    render(<OnboardingConnectProviderPage />);

    const input = screen.getByTestId("provider-key") as HTMLInputElement;
    expect(input.placeholder).toBe("sk-ant-...");
  });
});
