/**
 * Acceptance test for
 * `fix-onboarding-wizard-step-back-loses-firsthire-prompt-and-api-key`.
 *
 * Pins the sessionStorage-scoped API-key persistence on the
 * connect-provider step:
 *
 *  1. Typing into the input writes the value into sessionStorage
 *     (not localStorage — closing the tab MUST clear it).
 *  2. Mounting with a value already in sessionStorage hydrates the
 *     input, so refresh + step-back don't force the user to paste
 *     again.
 *  3. Successful submit and skip both clear the sessionStorage entry,
 *     so a fresh wizard run doesn't carry the prior tab's key.
 *  4. The user-facing scope note ("stays in this browser tab") renders.
 */

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import ConnectProviderPage from "../page";

const API_KEY_SESSION_STORAGE_KEY = "skycot:onboarding:apiKey";

const pushMock = vi.fn();

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock, replace: vi.fn(), back: vi.fn() }),
  usePathname: () => "/onboarding/connect-provider",
}));

function installSyntheticStorage(target: "localStorage" | "sessionStorage") {
  const data = new Map<string, string>();
  Object.defineProperty(window, target, {
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
  installSyntheticStorage("localStorage");
  installSyntheticStorage("sessionStorage");
  useOnboardingDraft.getState().reset();
  pushMock.mockClear();
  global.fetch = vi.fn();
});

afterEach(() => {
  window.localStorage.clear();
  window.sessionStorage.clear();
  vi.restoreAllMocks();
});

describe("Connect-provider API key persistence", () => {
  it("renders the tab-scope security note next to the input", () => {
    render(wrap(<ConnectProviderPage />));
    expect(
      screen.getByTestId("onboarding-provider-key-scope-note"),
    ).toHaveTextContent(/stays in this browser tab/i);
  });

  it("writes each keystroke into sessionStorage (NOT localStorage)", async () => {
    const user = userEvent.setup();
    render(wrap(<ConnectProviderPage />));

    await user.type(screen.getByTestId("provider-key"), "sk-ant-mid-edit");

    await waitFor(() => {
      expect(window.sessionStorage.getItem(API_KEY_SESSION_STORAGE_KEY)).toBe(
        "sk-ant-mid-edit",
      );
    });
    expect(window.localStorage.getItem(API_KEY_SESSION_STORAGE_KEY)).toBeNull();
  });

  it("hydrates the input from sessionStorage on mount", () => {
    window.sessionStorage.setItem(
      API_KEY_SESSION_STORAGE_KEY,
      "sk-ant-restored",
    );

    render(wrap(<ConnectProviderPage />));

    expect(screen.getByTestId("provider-key")).toHaveValue("sk-ant-restored");
  });

  it("clears the persisted key after a successful submit", async () => {
    const user = userEvent.setup();
    global.fetch = vi.fn(async () =>
      Response.json({ validated: true, test_cost: 0.001 }),
    ) as unknown as typeof fetch;

    render(wrap(<ConnectProviderPage />));
    await user.type(screen.getByTestId("provider-key"), "sk-ant-test-123");
    expect(window.sessionStorage.getItem(API_KEY_SESSION_STORAGE_KEY)).toBe(
      "sk-ant-test-123",
    );

    await user.click(screen.getByTestId("onboarding-provider-submit"));
    await waitFor(() =>
      expect(pushMock).toHaveBeenCalledWith("/onboarding/pick-vertical"),
    );
    expect(window.sessionStorage.getItem(API_KEY_SESSION_STORAGE_KEY)).toBeNull();
  });

  it("clears the persisted key when the user chooses managed keys", async () => {
    // Item-1 managed-keys default (2026-06-19): the non-demo escape is now
    // "Use Skycot managed keys" (the mock-provider skip is demo-only). It
    // clears any mid-edit BYOK key the same way the old skip did.
    const user = userEvent.setup();
    render(wrap(<ConnectProviderPage />));

    await user.type(screen.getByTestId("provider-key"), "sk-ant-mid-edit");
    expect(window.sessionStorage.getItem(API_KEY_SESSION_STORAGE_KEY)).toBe(
      "sk-ant-mid-edit",
    );

    await user.click(screen.getByTestId("onboarding-provider-use-managed"));
    expect(pushMock).toHaveBeenCalledWith("/onboarding/pick-vertical");
    expect(window.sessionStorage.getItem(API_KEY_SESSION_STORAGE_KEY)).toBeNull();
  });
});
