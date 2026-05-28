/**
 * Workspace step (onboarding step 2) — Path B advance behavior.
 *
 * Covers `fix-onboarding-workspace-step-celebration-hold-timer-jarring`
 * (onboarding F08 from UX e2e audit 2026-05-28-0252):
 *
 * - On a fresh successful provision the celebration toast fires but
 *   the page does NOT auto-navigate; the Continue button receives
 *   focus so pressing Enter advances cleanly.
 * - Returning users (workspace already in the persisted draft) see a
 *   "welcome back" banner and auto-advance within ~300ms — much
 *   shorter than the 1200ms hold the brief replaces.
 */

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createElement, type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import OnboardingWorkspacePage from "@/app/(setup)/onboarding/workspace/page";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";

const pushMock = vi.hoisted(() => vi.fn());
const toastSuccessMock = vi.hoisted(() => vi.fn());
const provisionMock = vi.hoisted(() => vi.fn());

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock, replace: vi.fn(), back: vi.fn() }),
  usePathname: () => "/onboarding/workspace",
}));

vi.mock("sonner", () => ({
  toast: { success: toastSuccessMock, error: vi.fn() },
}));

vi.mock("@/components/ui/sonner", () => ({
  Toaster: () => null,
}));

vi.mock("@/lib/onboarding/use-provision", () => ({
  useProvision: () => ({
    mutateAsync: provisionMock,
    isPending: false,
  }),
}));

function installStorage() {
  const data = new Map<string, string>();
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: {
      get length() {
        return data.size;
      },
      clear: () => data.clear(),
      getItem: (k: string) => (data.has(k) ? (data.get(k) as string) : null),
      key: (i: number) => Array.from(data.keys())[i] ?? null,
      removeItem: (k: string) => {
        data.delete(k);
      },
      setItem: (k: string, v: string) => {
        data.set(k, String(v));
      },
    },
  });
}

function wrap(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: {
      mutations: { retry: false },
      queries: { gcTime: 0, retry: false },
    },
  });
  return createElement(QueryClientProvider, { client }, ui);
}

beforeEach(() => {
  installStorage();
  pushMock.mockReset();
  toastSuccessMock.mockReset();
  provisionMock.mockReset();
  useOnboardingDraft.getState().reset();
});

afterEach(() => {
  window.localStorage.clear();
});

describe("workspace step — Path B manual advance", () => {
  it("does not auto-navigate after a fresh provision; focuses Continue and advances on Enter", async () => {
    provisionMock.mockResolvedValueOnce({
      tenantId: "tenant_a",
      workspaceId: "workspace_a",
      created: true,
    });

    const user = userEvent.setup();
    render(wrap(createElement(OnboardingWorkspacePage)));

    await user.type(screen.getByTestId("workspace-name"), "Acme Growth");
    await user.type(screen.getByTestId("company-name"), "Acme Inc.");
    await user.click(screen.getByTestId("onboarding-workspace-submit"));

    await waitFor(() => expect(toastSuccessMock).toHaveBeenCalledTimes(1));
    expect(toastSuccessMock).toHaveBeenCalledWith(
      "Workspace created — welcome to Skycot.",
    );

    const continueBtn = screen.getByTestId("onboarding-workspace-submit");
    await waitFor(() => expect(continueBtn).toHaveFocus());

    expect(pushMock).not.toHaveBeenCalled();
    expect(
      screen.getByTestId("onboarding-workspace-press-enter-hint"),
    ).toBeInTheDocument();

    await user.keyboard("{Enter}");
    expect(pushMock).toHaveBeenCalledWith("/onboarding/connect-provider");
  });

  it("idempotent re-provision (created=false) navigates immediately without celebrating", async () => {
    provisionMock.mockResolvedValueOnce({
      tenantId: "tenant_a",
      workspaceId: "workspace_a",
      created: false,
    });

    const user = userEvent.setup();
    render(wrap(createElement(OnboardingWorkspacePage)));

    await user.type(screen.getByTestId("workspace-name"), "Acme Growth");
    await user.type(screen.getByTestId("company-name"), "Acme Inc.");
    await user.click(screen.getByTestId("onboarding-workspace-submit"));

    await waitFor(() =>
      expect(pushMock).toHaveBeenCalledWith("/onboarding/connect-provider"),
    );
    expect(toastSuccessMock).not.toHaveBeenCalled();
  });

  it("returning user (workspace already provisioned) sees the banner and auto-advances within 300ms", async () => {
    useOnboardingDraft.getState().setProvision("tenant_a", "workspace_a");

    vi.useFakeTimers();
    try {
      render(wrap(createElement(OnboardingWorkspacePage)));

      expect(
        screen.getByTestId("onboarding-workspace-returning-banner"),
      ).toHaveTextContent(/welcome back/i);
      expect(pushMock).not.toHaveBeenCalled();

      await act(async () => {
        vi.advanceTimersByTime(250);
      });

      expect(pushMock).toHaveBeenCalledWith("/onboarding/connect-provider");
    } finally {
      vi.useRealTimers();
    }
  });
});
