/**
 * CTA hierarchy on /onboarding/connect-provider.
 *
 * Updated for Item-1 managed-keys default (operator 2026-06-19): the
 * NON-DEMO default now offers "Use Skycot managed keys" as the primary
 * escape (no API key, real responses) — the mock-provider "Skip for now"
 * + its warning are gated to DEMO mode only, so a real user can never be
 * dropped onto a simulated teammate. The original skip-vs-continue
 * hierarchy assertion (fix-onboarding-connect-provider-skip-button-
 * equal-weight-to-continue, onboarding F09) is preserved but re-scoped to
 * the demo path, where the skip button still exists.
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

describe("onboarding connect-provider — CTA hierarchy", () => {
  beforeEach(() => {
    providerKeyMock.mutateAsync.mockReset();
    providerKeyMock.isPending = false;
    demoModeMock.isDemoMode.mockReturnValue(false);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("non-demo: offers managed keys as the primary escape, with NO mock-provider skip", () => {
    render(<OnboardingConnectProviderPage />);

    // Managed keys is the recommended one-click primary CTA.
    const managed = screen.getByTestId("onboarding-provider-use-managed");
    expect(managed.className).toContain("ob-cta");
    expect(managed.className).not.toContain("ob-cta-ghost");

    // The BYOK "Validate & continue" submit is still present (secondary path).
    expect(screen.getByTestId("onboarding-provider-submit")).toBeInTheDocument();

    // A real user must NOT be offered the mock-provider skip or its warning.
    expect(screen.queryByTestId("onboarding-provider-skip")).toBeNull();
    expect(screen.queryByTestId("onboarding-provider-skip-warning")).toBeNull();
  });

  it("non-demo: the BYOK submit stays the filled primary even after a preferSkip-coded error", async () => {
    providerKeyMock.mutateAsync.mockRejectedValueOnce(
      new apiClientMock.ApiError(500, "anything", "provider_validation_unconfigured"),
    );

    render(<OnboardingConnectProviderPage />);

    const input = screen.getByTestId("provider-key") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "sk-ant-abc123" } });
    fireEvent.submit(screen.getByTestId("onboarding-connect-provider-form"));
    await screen.findByTestId("onboarding-provider-error");

    const submit = screen.getByTestId("onboarding-provider-submit");
    expect(submit.className).toContain("ob-cta");
    expect(submit.className).not.toContain("ob-cta-ghost");
    // Still no mock-skip in non-demo even when an error hint preferred it.
    expect(screen.queryByTestId("onboarding-provider-skip")).toBeNull();
  });

  it("demo: keeps Skip as the ghost + the mock-provider warning (the original F09 hierarchy)", () => {
    demoModeMock.isDemoMode.mockReturnValue(true);
    render(<OnboardingConnectProviderPage />);

    const submit = screen.getByTestId("onboarding-provider-submit");
    const skip = screen.getByTestId("onboarding-provider-skip");
    expect(submit.className).toContain("ob-cta");
    expect(submit.className).not.toContain("ob-cta-ghost");
    expect(skip.className).toContain("ob-cta-ghost");
    expect(skip.className).not.toMatch(/(^|\s)ob-cta(\s|$)/);

    const warning = screen.getByTestId("onboarding-provider-skip-warning");
    expect(warning.textContent ?? "").toMatch(/Mock provider/);
    // Managed-keys CTA is hidden in demo (mock is the demo path).
    expect(screen.queryByTestId("onboarding-provider-use-managed")).toBeNull();
  });
});
