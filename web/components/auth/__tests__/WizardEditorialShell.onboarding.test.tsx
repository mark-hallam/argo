import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import WelcomePage from "@/app/(setup)/onboarding/welcome/page";
import WorkspacePage from "@/app/(setup)/onboarding/workspace/page";
import ConnectProviderPage from "@/app/(setup)/onboarding/connect-provider/page";
import PickVerticalPage from "@/app/(setup)/onboarding/pick-vertical/page";
import FirstHirePage from "@/app/(setup)/onboarding/first-hire/page";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), back: vi.fn() }),
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

function wrap(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
  return <QueryClientProvider client={client}>{ui}</QueryClientProvider>;
}

beforeEach(() => {
  installStorage();
  useOnboardingDraft.getState().reset();
});

describe("Onboarding adoption of WizardEditorialShell", () => {
  it("welcome step renders the shell chrome + mission-pillar copy", () => {
    render(wrap(<WelcomePage />));
    expect(screen.getByTestId("wiz-shell-hero")).toBeInTheDocument();
    expect(screen.getByTestId("wiz-shell-eyebrow")).toBeInTheDocument();
    expect(screen.getByTestId("wiz-shell-trust")).toBeInTheDocument();
    expect(screen.getByTestId("wiz-shell-brand")).toBeInTheDocument();
    expect(screen.getByTestId("wiz-shell-exit")).toBeInTheDocument();
    expect(screen.getByTestId("onboarding-pillar-audit-first")).toBeInTheDocument();
    expect(screen.getByTestId("onboarding-pillar-single-binary")).toBeInTheDocument();
    expect(screen.getByTestId("onboarding-pillar-enterprise-grade")).toBeInTheDocument();
    const progress = screen.getByRole("progressbar");
    expect(progress).toHaveAttribute("aria-valuenow", "1");
    expect(progress).toHaveAttribute("aria-valuemax", "5");
  });

  it("workspace step reports step 2 of 5 on the shell progress strip", () => {
    render(wrap(<WorkspacePage />));
    const progress = screen.getByRole("progressbar");
    expect(progress).toHaveAttribute("aria-valuenow", "2");
    expect(progress).toHaveAttribute("aria-valuemax", "5");
    expect(screen.getByTestId("wiz-shell-trust")).toBeInTheDocument();
  });

  it("connect-provider step reports step 3 of 5 on the shell progress strip", () => {
    render(wrap(<ConnectProviderPage />));
    const progress = screen.getByRole("progressbar");
    expect(progress).toHaveAttribute("aria-valuenow", "3");
    expect(progress).toHaveAttribute("aria-valuemax", "5");
    expect(screen.getByTestId("wiz-shell-trust")).toBeInTheDocument();
  });

  it("pick-vertical step reports step 4 of 5 on the shell progress strip", () => {
    render(wrap(<PickVerticalPage />));
    const progress = screen.getByRole("progressbar");
    expect(progress).toHaveAttribute("aria-valuenow", "4");
    expect(progress).toHaveAttribute("aria-valuemax", "5");
    expect(screen.getByTestId("wiz-shell-trust")).toBeInTheDocument();
  });

  it("first-hire step reports step 5 of 5 on the shell progress strip", () => {
    render(wrap(<FirstHirePage />));
    const progress = screen.getByRole("progressbar");
    expect(progress).toHaveAttribute("aria-valuenow", "5");
    expect(progress).toHaveAttribute("aria-valuemax", "5");
    expect(screen.getByTestId("wiz-shell-trust")).toBeInTheDocument();
  });
});
