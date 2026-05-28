/**
 * Acceptance test for the canonical-roster picker on step 5
 * (replaces the prior plain-English textarea per
 * `fix-onboarding-first-hire-canonical-picker-not-textarea`).
 *
 * The old textarea-and-typed-name UX was demo theater: in demo mode
 * the SPA `firstHire()` mutation ignores the description entirely and
 * routes to a canonical sample worker keyed by `verticalId`. The
 * picker now shows the matched canonical worker up-front so what the
 * camera reads on the card matches what the post-hire profile shows.
 *
 * Pins:
 *  1. The page renders the canonical persona matched to `verticalId`
 *     in the onboarding draft store.
 *  2. The hero, sub-copy, and submit button surface the matched
 *     persona's name (no generic "Hire them" fallback when a match
 *     exists).
 *  3. Unknown verticals fall back to Mariah (the runbook's canonical
 *     "first hire" placeholder).
 */

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
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

describe("First-hire canonical-roster picker", () => {
  it("renders the matched canonical persona for the picked vertical (customer-support → Jamal)", () => {
    useOnboardingDraft.getState().setVertical("customer-support");
    render(wrap(<FirstHirePage />));

    // Hero names the matched persona.
    expect(screen.getByText(/Meet/i)).toBeInTheDocument();
    expect(screen.getByText(/Jamal\./)).toBeInTheDocument();

    // Card surfaces persona content (traits + greatAt + escalateWhen).
    expect(screen.getByText("Calm under pressure")).toBeInTheDocument();
    expect(screen.getByText(/Customer mentions churn/i)).toBeInTheDocument();

    // Submit button uses the persona's name.
    expect(
      screen.getByTestId("onboarding-first-hire-submit"),
    ).toHaveTextContent(/Hire Jamal/);
  });

  it("matches marketing-intern → Nori", () => {
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));

    expect(screen.getByText(/Nori\./)).toBeInTheDocument();
    expect(
      screen.getByTestId("onboarding-first-hire-submit"),
    ).toHaveTextContent(/Hire Nori/);
  });

  it("falls back to Mariah when verticalId has no canonical match", () => {
    // No setVertical call → verticalId stays empty in the draft store.
    render(wrap(<FirstHirePage />));

    expect(screen.getByText(/Mariah\./)).toBeInTheDocument();
    expect(
      screen.getByTestId("onboarding-first-hire-submit"),
    ).toHaveTextContent(/Hire Mariah/);
  });

  it("uses the new STEP 05 · YOUR FIRST TEAMMATE eyebrow (no 'worker' vocab)", () => {
    useOnboardingDraft.getState().setVertical("customer-support");
    render(wrap(<FirstHirePage />));

    expect(screen.getByText(/STEP 05 · YOUR FIRST TEAMMATE/i)).toBeInTheDocument();
    // The old "Describe your first worker" hero must not appear.
    expect(screen.queryByText(/Describe your first worker/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/first worker/i)).not.toBeInTheDocument();
  });
});
