import { render, screen, act } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WizardEditorialShell } from "@/components/auth/WizardEditorialShell";

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), back: vi.fn() }),
  usePathname: () => "/onboarding/welcome",
}));

function installSessionStorage() {
  const data = new Map<string, string>();
  Object.defineProperty(window, "sessionStorage", {
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

function renderShell() {
  return render(
    <WizardEditorialShell
      step={1}
      stepLabel="WELCOME"
      heroSegments={[{ text: "Hello" }]}
    >
      <div>body</div>
    </WizardEditorialShell>,
  );
}

describe("WizardEditorialShell demo-mode banner pass-through", () => {
  const originalDemoMode = process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE;

  beforeEach(() => {
    installSessionStorage();
  });

  afterEach(() => {
    if (originalDemoMode === undefined) {
      delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE;
    } else {
      process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE = originalDemoMode;
    }
  });

  it("renders the demo-mode banner when NEXT_PUBLIC_SKYCOT_DEMO_MODE=1", () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE = "1";
    renderShell();
    const banner = screen.getByTestId("demo-mode-banner");
    expect(banner).toBeInTheDocument();
    expect(banner.textContent).toContain("Demo mode");
    expect(banner.textContent).toContain("staged teammate");
    expect(banner.textContent).toContain("simulated");
  });

  it("does not render the demo-mode banner when the env var is unset", () => {
    delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE;
    renderShell();
    expect(screen.queryByTestId("demo-mode-banner")).not.toBeInTheDocument();
  });

  it("does not render the demo-mode banner when the env var is 0", () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE = "0";
    renderShell();
    expect(screen.queryByTestId("demo-mode-banner")).not.toBeInTheDocument();
  });

  it("hides the banner after dismissal and persists the choice in sessionStorage", () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE = "1";
    renderShell();
    const dismiss = screen.getByTestId("demo-mode-banner-dismiss");
    act(() => {
      dismiss.click();
    });
    expect(screen.queryByTestId("demo-mode-banner")).not.toBeInTheDocument();
    expect(window.sessionStorage.getItem("skycot:demo-mode-banner-dismissed")).toBe("1");
  });

  it("stays dismissed across remounts when sessionStorage carries the flag", () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE = "1";
    window.sessionStorage.setItem("skycot:demo-mode-banner-dismissed", "1");
    renderShell();
    expect(screen.queryByTestId("demo-mode-banner")).not.toBeInTheDocument();
  });
});
