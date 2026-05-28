/**
 * Acceptance test for
 * `fix-onboarding-wizard-step-back-loses-firsthire-prompt-and-api-key`.
 *
 * Pins step-back / refresh survival of the first-hire prompt:
 *
 *  1. Typing into the textarea flows into the draft store on every
 *     keystroke (no debounce — `localStorage` is the durable
 *     persistence layer).
 *  2. The textarea hydrates from `useOnboardingDraft.firstHirePrompt`
 *     on mount, so a user who navigates pick-vertical → first-hire →
 *     pick-vertical → first-hire sees what they typed.
 */

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import FirstHirePage from "../page";

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), back: vi.fn() }),
  usePathname: () => "/onboarding/first-hire",
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
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  return <QueryClientProvider client={client}>{ui}</QueryClientProvider>;
}

beforeEach(() => {
  installStorage();
  useOnboardingDraft.getState().reset();
});

afterEach(() => {
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe("First-hire prompt persistence", () => {
  it("writes every keystroke into the draft store (no debounce)", async () => {
    const user = userEvent.setup();
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));

    const textarea = screen.getByTestId("first-hire-prompt");
    await user.type(textarea, "Maya the intern");

    await waitFor(() => {
      expect(useOnboardingDraft.getState().firstHirePrompt).toBe(
        "Maya the intern",
      );
    });
  });

  it("hydrates the textarea from the draft store on mount", () => {
    useOnboardingDraft.getState().setVertical("marketing-intern");
    useOnboardingDraft
      .getState()
      .setFirstHirePrompt("Half-typed prompt from a prior visit");

    render(wrap(<FirstHirePage />));

    expect(screen.getByTestId("first-hire-prompt")).toHaveValue(
      "Half-typed prompt from a prior visit",
    );
  });
});
