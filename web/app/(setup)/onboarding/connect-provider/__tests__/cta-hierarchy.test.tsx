/**
 * Acceptance test for
 * `fix-onboarding-connect-provider-skip-button-equal-weight-to-continue`.
 *
 * Pins the CTA hierarchy on /onboarding/connect-provider so that the
 * "Validate & continue" button always renders as the filled primary
 * (`ob-cta`) and "Skip for now" always renders as the outline-ghost
 * (`ob-cta-ghost`) — even after a validation error whose previous
 * recovery hint preferred skipping. Closes onboarding F09 (UX e2e
 * audit 2026-05-28-0252).
 */

import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const routerMock = vi.hoisted(() => ({
  push: vi.fn(),
  replace: vi.fn(),
  back: vi.fn(),
}));

const draftMock = vi.hoisted(() => {
  const state = {
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

describe("onboarding connect-provider — CTA hierarchy", () => {
  beforeEach(() => {
    providerKeyMock.mutateAsync.mockReset();
    providerKeyMock.isPending = false;
    demoModeMock.isDemoMode.mockReturnValue(false);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders Continue as the filled primary and Skip as the ghost in the default state", () => {
    render(<OnboardingConnectProviderPage />);

    const submit = screen.getByTestId("onboarding-provider-submit");
    const skip = screen.getByTestId("onboarding-provider-skip");

    expect(submit.className).toContain("ob-cta");
    expect(submit.className).not.toContain("ob-cta-ghost");
    expect(skip.className).toContain("ob-cta-ghost");
    expect(skip.className).not.toMatch(/(^|\s)ob-cta(\s|$)/);
  });

  it("keeps Continue as the primary CTA even when validation fails with a preferSkip-coded error", async () => {
    providerKeyMock.mutateAsync.mockRejectedValueOnce(
      new apiClientMock.ApiError(500, "anything", "provider_validation_unconfigured"),
    );

    render(<OnboardingConnectProviderPage />);

    const input = screen.getByTestId("provider-key") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "sk-ant-abc123" } });
    const form = screen.getByTestId("onboarding-connect-provider-form");
    fireEvent.submit(form);

    // Wait for the rejection + state update to flush.
    await screen.findByTestId("onboarding-provider-error");

    const submit = screen.getByTestId("onboarding-provider-submit");
    const skip = screen.getByTestId("onboarding-provider-skip");

    // The CTA hierarchy must NOT invert when the error suggested skipping.
    expect(submit.className).toContain("ob-cta");
    expect(submit.className).not.toContain("ob-cta-ghost");
    expect(skip.className).toContain("ob-cta-ghost");
    expect(skip.className).not.toMatch(/(^|\s)ob-cta(\s|$)/);
  });

  it("still shows the mock-provider warning callout above the buttons", () => {
    render(<OnboardingConnectProviderPage />);

    const warning = screen.getByTestId("onboarding-provider-skip-warning");
    expect(warning).toBeInTheDocument();
    expect(warning.textContent ?? "").toMatch(/Mock provider/);
    expect(warning.textContent ?? "").toMatch(/simulated/i);
  });
});
