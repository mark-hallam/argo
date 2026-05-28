import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { createElement, type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import FirstHirePage from "@/app/(setup)/onboarding/first-hire/page";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";

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
      mutations: { retry: false },
      queries: { gcTime: 0, retry: false },
    },
  });
  return createElement(QueryClientProvider, { client }, ui);
}

beforeEach(() => {
  installStorage();
  useOnboardingDraft.getState().reset();
});

afterEach(() => {
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe("first-hire copy honesty (Phase 1)", () => {
  it("does not promise schedule extraction in the sub-copy", () => {
    render(wrap(createElement(FirstHirePage)));

    // Phase 1: the wizard only sends {description, vertical_id} — no cron
    // is parsed out — so the copy must not promise "this week" cadence.
    const heading = screen.getByText(/Describe your/i);
    const card = heading.closest("section, div") ?? document.body;
    expect(card.textContent).not.toMatch(/this week/i);
    expect(card.textContent).toMatch(/name and a role/i);
    expect(card.textContent).toMatch(/set their schedule after they're hired/i);
  });

  it("placeholder example doesn't mention weekly/daily cadence", () => {
    render(wrap(createElement(FirstHirePage)));

    const textarea = screen.getByTestId("first-hire-prompt") as HTMLTextAreaElement;
    const placeholder = textarea.placeholder;
    expect(placeholder).not.toMatch(/weekly|daily/i);
    // Existing example name shape preserved (marketing intern named Maya)
    // so visual regression on the wizard stays minimal.
    expect(placeholder).toMatch(/Maya/);
  });
});
