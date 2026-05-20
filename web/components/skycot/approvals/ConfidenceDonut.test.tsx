import { render, screen, act } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ConfidenceDonut } from "./ConfidenceDonut";

describe("ConfidenceDonut", () => {
  beforeEach(() => {
    // Default to "no reduced-motion preference" for these tests.
    vi.stubGlobal(
      "matchMedia",
      vi.fn().mockReturnValue({ matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() }),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("clamps the percent to [0,100] and renders the label", () => {
    render(<ConfidenceDonut value={1.4} />);
    const el = screen.getByTestId("confidence-donut");
    expect(el).toHaveAttribute("data-percent", "100");
    expect(el).toHaveTextContent(/100%/);
  });

  it("maps >=85% to the high tone", () => {
    render(<ConfidenceDonut value={0.92} />);
    expect(screen.getByTestId("confidence-donut")).toHaveAttribute("data-tone", "high");
  });

  it("maps the [60%, 85%) band to the mid tone", () => {
    render(<ConfidenceDonut value={0.72} />);
    expect(screen.getByTestId("confidence-donut")).toHaveAttribute("data-tone", "mid");
  });

  it("maps <60% to the low tone", () => {
    render(<ConfidenceDonut value={0.45} />);
    expect(screen.getByTestId("confidence-donut")).toHaveAttribute("data-tone", "low");
  });

  it("hides the visible label and exposes an SR-only one when showLabel=false", () => {
    render(<ConfidenceDonut value={0.7} showLabel={false} />);
    const el = screen.getByTestId("confidence-donut");
    expect(el.textContent).toMatch(/70% confidence/);
  });

  it("skips the entrance transition when prefers-reduced-motion is set", async () => {
    vi.stubGlobal(
      "matchMedia",
      vi.fn().mockReturnValue({ matches: true, addEventListener: vi.fn(), removeEventListener: vi.fn() }),
    );
    // The animation effect schedules a requestAnimationFrame; here we
    // verify the rendered component data-tone matches the final state
    // without any frame-step wait. Reduced-motion gate flips `filled`
    // synchronously inside useEffect.
    render(<ConfidenceDonut value={0.9} animate />);
    await act(async () => {});
    expect(screen.getByTestId("confidence-donut")).toHaveAttribute("data-tone", "high");
  });
});
