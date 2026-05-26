import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createElement, type ReactNode } from "react";

import OnboardingConnectProviderPage from "@/app/(setup)/onboarding/connect-provider/page";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";

const pushMock = vi.fn();
const backMock = vi.fn();

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: pushMock,
    back: backMock,
    replace: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    forward: vi.fn(),
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
      getItem: (key: string) => (data.has(key) ? (data.get(key) as string) : null),
      key: (index: number) => Array.from(data.keys())[index] ?? null,
      removeItem: (key: string) => {
        data.delete(key);
      },
      setItem: (key: string, value: string) => {
        data.set(key, String(value));
      },
    },
  });
}

function renderWithQuery(children: ReactNode) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  return render(createElement(QueryClientProvider, { client }, children));
}

describe("OnboardingConnectProviderPage", () => {
  beforeEach(() => {
    installStorage();
    useOnboardingDraft.getState().reset();
    pushMock.mockReset();
    backMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders the mock-provider warning above the Skip button", () => {
    renderWithQuery(createElement(OnboardingConnectProviderPage));

    const warning = screen.getByTestId("onboarding-provider-skip-warning");
    const skip = screen.getByTestId("onboarding-provider-skip");

    expect(warning).toBeVisible();
    expect(warning).toHaveTextContent("Mock provider — workers run but responses are simulated.");
    expect(warning).toHaveClass("ob-form-note");
    expect(Boolean(warning.compareDocumentPosition(skip) & Node.DOCUMENT_POSITION_FOLLOWING)).toBe(
      true,
    );
  });
});
