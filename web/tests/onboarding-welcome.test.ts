import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import WelcomePage from "@/app/(setup)/onboarding/welcome/page";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";

const pushMock = vi.hoisted(() => vi.fn());

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock, replace: vi.fn(), back: vi.fn() }),
  usePathname: () => "/onboarding/welcome",
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

beforeEach(() => {
  installStorage();
  pushMock.mockClear();
  useOnboardingDraft.getState().reset();
});

afterEach(() => {
  window.localStorage.clear();
});

describe("onboarding welcome Genesis framing", () => {
  it("renders onboarding-studio copy and still advances to workspace", async () => {
    const user = userEvent.setup();
    render(createElement(WelcomePage));

    expect(screen.getByTestId("wiz-shell-hero")).toHaveTextContent(
      "Hire your first teammate.",
    );
    expect(screen.getByText(/onboard your first AI teammate/i)).toBeInTheDocument();
    expect(screen.getByTestId("onboarding-pillar-genesis-studio")).toHaveTextContent(
      /plain English/i,
    );
    expect(screen.getByTestId("onboarding-pillar-genesis-studio")).toHaveTextContent(
      /charter, action vocabulary, memory schema, and eval suites/i,
    );
    expect(screen.getByTestId("onboarding-pillar-genesis-studio")).toHaveTextContent(
      /Voice and SOP uploads coming next/i,
    );

    await user.click(screen.getByTestId("onboarding-welcome-continue"));
    expect(pushMock).toHaveBeenCalledWith("/onboarding/workspace");
  });
});
