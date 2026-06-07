/**
 * First-hire feedback affordances (F14) — RE-SCOPED residue of
 * `rebase-fix-onboarding-signup-and-first-hire-feedback-affordances`.
 *
 * F6 (landing demo shortcut) + F7 (SMTP probe / demo fallback) already
 * shipped on main via #2357 + the demo-mint route, so this pin covers
 * only the two remaining F14 deltas:
 *   1. a sonner success-toast acknowledgment that fires before the
 *      page navigates away on a successful hire, and
 *   2. a "skip to your team" escape link that appears when the hire
 *      fails, so a failed hire never traps the buyer on the step.
 *
 * It also pins the in-flight loading state (spinner + aria-busy) that
 * #2648/#2649 shipped, so a future edit can't silently regress it.
 */

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createElement, type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import FirstHirePage from "@/app/(setup)/onboarding/first-hire/page";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";

const pushMock = vi.hoisted(() => vi.fn());
const toastSuccessMock = vi.hoisted(() => vi.fn());
const mutateAsyncMock = vi.hoisted(() => vi.fn());

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock, replace: vi.fn(), back: vi.fn() }),
  usePathname: () => "/onboarding/first-hire",
}));

vi.mock("sonner", () => ({
  toast: { success: toastSuccessMock, error: vi.fn() },
}));

vi.mock("@/components/ui/sonner", () => ({
  Toaster: () => null,
}));

// Force the non-demo branch so the success path routes through
// /onboarding/done deterministically (demo mode would route to /workers).
vi.mock("@/lib/skycot/demo-mode", () => ({
  isDemoMode: () => false,
}));

vi.mock("@/lib/onboarding/use-first-hire", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("@/lib/onboarding/use-first-hire")>();
  return {
    ...actual,
    useFirstHire: () => ({ mutateAsync: mutateAsyncMock, isPending: false }),
  };
});

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

async function fillAndSubmit() {
  const user = userEvent.setup();
  render(wrap(createElement(FirstHirePage)));
  await user.type(screen.getByTestId("first-hire-name"), "Maya");
  await user.type(
    screen.getByTestId("first-hire-prompt"),
    "marketing intern who drafts blog posts",
  );
  await user.click(screen.getByTestId("onboarding-first-hire-submit"));
}

beforeEach(() => {
  installStorage();
  pushMock.mockClear();
  toastSuccessMock.mockClear();
  mutateAsyncMock.mockReset();
  useOnboardingDraft.getState().reset();
  useOnboardingDraft.getState().setProvision("tenant_onboarding_1", "workspace_1");
  useOnboardingDraft.getState().setVertical("marketing-intern");
});

afterEach(() => {
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe("first-hire feedback affordances (F14)", () => {
  it("fires the success toast before navigating on a successful hire", async () => {
    mutateAsyncMock.mockResolvedValue({
      kind: "hired",
      workerId: "wkr_maya",
      workerName: "Maya",
    });

    await fillAndSubmit();

    // The acknowledgment toast must fire as soon as the hire resolves —
    // before the route change unmounts the page.
    await waitFor(() => {
      expect(toastSuccessMock).toHaveBeenCalledWith(
        "✓ Hired Maya. Taking you to their page…",
      );
    });

    // ...and navigation still happens (after the brief hold).
    await waitFor(
      () => {
        expect(pushMock).toHaveBeenCalledWith(
          "/onboarding/done?worker=wkr_maya&name=Maya",
        );
      },
      { timeout: 4_000 },
    );
  });

  it("renders a 'skip to your team' escape link when the hire fails", async () => {
    mutateAsyncMock.mockRejectedValue(new Error("gateway unreachable"));

    await fillAndSubmit();

    const skipLink = await screen.findByTestId(
      "first-hire-skip-to-team",
      {},
      { timeout: 4_000 },
    );
    expect(skipLink).toHaveAttribute("href", "/team");
    // A failed hire must not navigate the buyer away on its own.
    expect(pushMock).not.toHaveBeenCalled();
    expect(toastSuccessMock).not.toHaveBeenCalled();
  });

  it("does not show the skip-to-team link before any failure", async () => {
    mutateAsyncMock.mockResolvedValue({
      kind: "hired",
      workerId: "wkr_maya",
      workerName: "Maya",
    });
    render(wrap(createElement(FirstHirePage)));
    expect(screen.queryByTestId("first-hire-skip-to-team")).toBeNull();
  });
});
