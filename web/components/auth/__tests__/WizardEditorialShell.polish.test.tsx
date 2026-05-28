import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WizardEditorialShell } from "@/components/auth/WizardEditorialShell";
import { DemoBanner } from "@/components/skycot/DemoBanner";

// usePathname returns whatever value the test sets here on mount; the
// shell uses pathname as its useEffect dep so we can pin route-change
// behaviour by re-rendering with a different value (see F13 case).
let mockPathname = "/onboarding/welcome";

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), back: vi.fn() }),
  usePathname: () => mockPathname,
}));

vi.mock("@/lib/skycot/demo-mode", () => ({
  isDemoMode: () => true,
}));

beforeEach(() => {
  mockPathname = "/onboarding/welcome";
  window.scrollTo = vi.fn();
  try {
    window.localStorage.clear();
  } catch {
    /* jsdom default storage */
  }
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("WizardEditorialShell polish cluster (F13 + F14)", () => {
  it("F13: scroll resets to top on mount", () => {
    const scrollTo = vi.fn();
    window.scrollTo = scrollTo;
    render(
      <WizardEditorialShell
        step={1}
        stepLabel="WELCOME"
        heroSegments={[{ text: "Hello" }]}
      />,
    );
    expect(scrollTo).toHaveBeenCalledWith(0, 0);
  });

  it("F13: scroll re-fires when pathname dep changes", () => {
    const scrollTo = vi.fn();
    window.scrollTo = scrollTo;
    const { rerender } = render(
      <WizardEditorialShell
        step={1}
        stepLabel="WELCOME"
        heroSegments={[{ text: "Hello" }]}
      />,
    );
    expect(scrollTo).toHaveBeenCalledTimes(1);
    mockPathname = "/onboarding/workspace";
    rerender(
      <WizardEditorialShell
        step={2}
        stepLabel="WORKSPACE"
        heroSegments={[{ text: "Workspace" }]}
      />,
    );
    expect(scrollTo).toHaveBeenCalledTimes(2);
    expect(scrollTo).toHaveBeenLastCalledWith(0, 0);
  });
});

describe("DemoBanner pathname gate (F14)", () => {
  it("renders on non-onboarding routes when demo-mode is on", () => {
    mockPathname = "/cohorts";
    render(<DemoBanner />);
    expect(screen.getByTestId("demo-banner")).toBeInTheDocument();
  });

  it("is suppressed on /onboarding/* routes (any viewport)", () => {
    mockPathname = "/onboarding/welcome";
    render(<DemoBanner />);
    expect(screen.queryByTestId("demo-banner")).not.toBeInTheDocument();
  });

  it("is suppressed on nested /onboarding routes", () => {
    mockPathname = "/onboarding/pick-vertical";
    render(<DemoBanner />);
    expect(screen.queryByTestId("demo-banner")).not.toBeInTheDocument();
  });
});
