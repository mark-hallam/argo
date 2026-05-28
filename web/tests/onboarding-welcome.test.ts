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
    const genesisPillar = screen.getByTestId("onboarding-pillar-genesis-studio");
    expect(genesisPillar).toHaveTextContent(/Genesis . Onboarding Studio/i);
    expect(genesisPillar).toHaveTextContent(/plain English/i);
    expect(genesisPillar).toHaveTextContent(
      /charter, action vocabulary, memory schema, and eval suites/i,
    );
    expect(genesisPillar).toHaveTextContent(
      /ready to start work in minutes/i,
    );
    // The "coming next" footnote buried Genesis as a stub. The flagship
    // framing reads as a present-tense capability.
    expect(genesisPillar.textContent ?? "").not.toMatch(/coming next/i);

    // The Genesis CTA is promoted to a tertiary button next to Get
    // Started, not a tiny underline below it.
    const genesisLink = screen.getByTestId("onboarding-genesis-link");
    expect(genesisLink.className).toMatch(/ob-cta-ghost/);
    expect(genesisLink.textContent ?? "").toMatch(/Genesis studio/);

    await user.click(screen.getByTestId("onboarding-welcome-continue"));
    expect(pushMock).toHaveBeenCalledWith("/onboarding/workspace");
  });
});
