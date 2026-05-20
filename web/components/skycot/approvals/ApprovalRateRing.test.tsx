import { render, screen, act } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApprovalRateRing } from "./ApprovalRateRing";

describe("ApprovalRateRing", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "matchMedia",
      vi.fn().mockReturnValue({ matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() }),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("rounds the slice shares to whole percents in the data attributes", () => {
    render(<ApprovalRateRing approved={0.78} refused={0.14} expired={0.08} />);
    const ring = screen.getByTestId("approval-rate-ring");
    expect(ring).toHaveAttribute("data-approved", "78");
    expect(ring).toHaveAttribute("data-refused", "14");
    expect(ring).toHaveAttribute("data-expired", "8");
  });

  it("renders the approved share as the center headline", () => {
    render(<ApprovalRateRing approved={0.78} refused={0.14} expired={0.08} />);
    expect(screen.getByTestId("approval-rate-ring")).toHaveTextContent("78%");
  });

  it("falls back to zero when the slices sum to zero", () => {
    render(<ApprovalRateRing approved={0} refused={0} expired={0} />);
    const ring = screen.getByTestId("approval-rate-ring");
    expect(ring).toHaveAttribute("data-approved", "0");
  });

  it("respects prefers-reduced-motion when animate=true", async () => {
    vi.stubGlobal(
      "matchMedia",
      vi.fn().mockReturnValue({ matches: true, addEventListener: vi.fn(), removeEventListener: vi.fn() }),
    );
    render(<ApprovalRateRing approved={0.78} refused={0.14} expired={0.08} animate />);
    await act(async () => {});
    // Reduced-motion users skip the transition phase and see the
    // final frame without a CSS transition attached.
    const ring = screen.getByTestId("approval-rate-ring");
    expect(ring.getAttribute("style") ?? "").not.toMatch(/transition:\s*background/);
  });
});
