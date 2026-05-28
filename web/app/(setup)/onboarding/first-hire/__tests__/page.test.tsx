/**
 * Acceptance test for
 * `fix-onboarding-first-hire-add-explicit-name-field-honest-parse`.
 *
 * Pins the F13 fix: the first-hire wizard now has a dedicated `name`
 * input alongside the description textarea. The page must:
 *
 *  1. Render both a name input and a description textarea.
 *  2. Disable submit until both inputs are non-empty.
 *  3. POST `{name, description, vertical_id}` to the gateway.
 *  4. Drop the "give them a name" copy from the description label,
 *     since name now has its own input.
 *  5. Persist firstHireName + firstHirePrompt across remounts via the
 *     Zustand draft store (re-pin from the prior persistence test).
 */

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import FirstHirePage from "../page";

const pushMock = vi.fn();

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock, replace: vi.fn(), back: vi.fn() }),
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
  pushMock.mockClear();
  global.fetch = vi.fn();
});

afterEach(() => {
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe("OnboardingFirstHirePage — explicit name field (F13)", () => {
  it("renders both a name input and a description textarea", () => {
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));
    expect(screen.getByTestId("first-hire-name")).toBeInTheDocument();
    expect(screen.getByTestId("first-hire-prompt")).toBeInTheDocument();
  });

  it("disables submit until both name and description are non-empty", async () => {
    const user = userEvent.setup();
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));
    const submit = screen.getByTestId("onboarding-first-hire-submit");
    expect(submit).toBeDisabled();
    await user.type(screen.getByTestId("first-hire-name"), "Maya");
    expect(submit).toBeDisabled();
    await user.type(
      screen.getByTestId("first-hire-prompt"),
      "marketing intern who drafts blog posts",
    );
    expect(submit).not.toBeDisabled();
  });

  it("posts {name, description, vertical_id} as separate fields", async () => {
    const user = userEvent.setup();
    useOnboardingDraft.getState().setVertical("marketing-intern");
    const fetchMock = vi.fn(async () =>
      Response.json({ worker_id: "wkr_maya_1", worker_name: "Maya" }),
    ) as unknown as typeof fetch;
    global.fetch = fetchMock;

    render(wrap(<FirstHirePage />));
    await user.type(screen.getByTestId("first-hire-name"), "Maya");
    await user.type(
      screen.getByTestId("first-hire-prompt"),
      "drafts blog posts and runs weekly competitive scans",
    );
    await user.click(screen.getByTestId("onboarding-first-hire-submit"));

    await waitFor(() =>
      expect(pushMock).toHaveBeenCalledWith("/workers/wkr_maya_1"),
    );

    const firstHireCall = (
      fetchMock as unknown as { mock: { calls: unknown[][] } }
    ).mock.calls.find((call) => {
      const url = call[0];
      return (
        typeof url === "string" &&
        url.includes("/api/v1/onboarding/first-hire")
      );
    });
    expect(firstHireCall).toBeDefined();
    const init = firstHireCall![1] as RequestInit;
    const body = JSON.parse(init.body as string) as Record<string, unknown>;
    expect(body.name).toBe("Maya");
    expect(body.description).toBe(
      "drafts blog posts and runs weekly competitive scans",
    );
    expect(body.vertical_id).toBe("marketing-intern");
  });

  it("description copy no longer claims to accept a name", () => {
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));
    // The hero subcopy used to say "Give them a name and a role". With
    // the dedicated name input the description label is for role +
    // tasks only — the "name" claim is gone from the surrounding copy.
    expect(document.body.textContent ?? "").not.toMatch(
      /give them a.*name/i,
    );
  });

  it("hydrates name + description from the draft store on mount", () => {
    useOnboardingDraft.getState().setVertical("marketing-intern");
    useOnboardingDraft.getState().setFirstHireName("Maya");
    useOnboardingDraft
      .getState()
      .setFirstHirePrompt("Half-typed prompt from a prior visit");

    render(wrap(<FirstHirePage />));
    expect(screen.getByTestId("first-hire-name")).toHaveValue("Maya");
    expect(screen.getByTestId("first-hire-prompt")).toHaveValue(
      "Half-typed prompt from a prior visit",
    );
  });

  it("writes every name keystroke into the draft store (no debounce)", async () => {
    const user = userEvent.setup();
    useOnboardingDraft.getState().setVertical("marketing-intern");
    render(wrap(<FirstHirePage />));

    await user.type(screen.getByTestId("first-hire-name"), "Marcus");

    await waitFor(() => {
      expect(useOnboardingDraft.getState().firstHireName).toBe("Marcus");
    });
  });
});
