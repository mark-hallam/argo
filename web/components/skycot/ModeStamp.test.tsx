import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TooltipProvider } from "@/components/ui/Tooltip";

const demoModeMock = vi.hoisted(() => ({
  isDemoMode: vi.fn((): boolean => false),
}));

vi.mock("@/lib/skycot/demo-mode", () => demoModeMock);

import { ModeStamp } from "./ModeStamp";

function renderStamp() {
  return render(
    <TooltipProvider>
      <ModeStamp />
    </TooltipProvider>,
  );
}

const originalNodeEnv = process.env.NODE_ENV;

beforeEach(() => {
  demoModeMock.isDemoMode.mockReset();
  demoModeMock.isDemoMode.mockReturnValue(false);
  (process.env as Record<string, string | undefined>).NODE_ENV = "development";
});

afterEach(() => {
  (process.env as Record<string, string | undefined>).NODE_ENV = originalNodeEnv;
});

describe("ModeStamp", () => {
  it("renders 'Live mode' with the green dot when demo mode is off", () => {
    demoModeMock.isDemoMode.mockReturnValue(false);
    renderStamp();
    const stamp = screen.getByTestId("sidebar-mode-stamp");
    expect(stamp).toHaveAttribute("data-mode", "live");
    expect(stamp).toHaveTextContent("Live mode");
    const dot = screen.getByTestId("sidebar-mode-stamp-dot");
    expect(dot.getAttribute("style") ?? "").toContain("#10b981");
  });

  it("renders 'Demo mode' with the purple dot when demo mode is on", () => {
    demoModeMock.isDemoMode.mockReturnValue(true);
    renderStamp();
    const stamp = screen.getByTestId("sidebar-mode-stamp");
    expect(stamp).toHaveAttribute("data-mode", "demo");
    expect(stamp).toHaveTextContent("Demo mode");
    const dot = screen.getByTestId("sidebar-mode-stamp-dot");
    expect(dot.getAttribute("style") ?? "").toContain("#8b5cf6");
  });

  it("renders nothing when NODE_ENV is production", () => {
    (process.env as Record<string, string | undefined>).NODE_ENV = "production";
    demoModeMock.isDemoMode.mockReturnValue(false);
    const { container } = renderStamp();
    expect(container.firstChild).toBeNull();
    expect(screen.queryByTestId("sidebar-mode-stamp")).toBeNull();
  });
});
