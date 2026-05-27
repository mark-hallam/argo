import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SLABar } from "./SLABar";

const NOW = new Date("2026-05-21T12:00:00Z").getTime();
const HOUR = 60 * 60 * 1000;

beforeEach(() => {
  vi.spyOn(Date, "now").mockReturnValue(NOW);
});

afterEach(() => {
  vi.restoreAllMocks();
});

function expiresIn(ms: number): string {
  return new Date(NOW + ms).toISOString();
}

describe("SLABar", () => {
  it("flips to the green tone when at least 4h remain", async () => {
    render(<SLABar expiresAt={expiresIn(8 * HOUR)} />);
    const bar = await screen.findByTestId("sla-bar");
    expect(bar).toHaveAttribute("data-tone", "ok");
    expect(bar).toHaveTextContent(/8h left/);
  });

  it("flips to the amber tone between 1h and 4h", async () => {
    render(<SLABar expiresAt={expiresIn(2 * HOUR)} />);
    const bar = await screen.findByTestId("sla-bar");
    expect(bar).toHaveAttribute("data-tone", "warn");
  });

  it("flips to the red tone with <1h remaining", async () => {
    render(<SLABar expiresAt={expiresIn(30 * 60 * 1000)} />);
    const bar = await screen.findByTestId("sla-bar");
    expect(bar).toHaveAttribute("data-tone", "crit");
    expect(bar).toHaveTextContent(/30m left/);
  });

  it("reports 'over' time once the deadline has passed", async () => {
    render(<SLABar expiresAt={expiresIn(-15 * 60 * 1000)} />);
    const bar = await screen.findByTestId("sla-bar");
    expect(bar).toHaveAttribute("data-tone", "crit");
    expect(bar).toHaveTextContent(/over 15m/);
  });

  it("exposes the remaining time on the progress role for screen readers", async () => {
    render(<SLABar expiresAt={expiresIn(8 * HOUR)} windowMs={24 * HOUR} />);
    const progress = await screen.findByRole("progressbar");
    expect(progress).toHaveAttribute("aria-valuenow");
    expect(progress).toHaveAttribute("aria-valuemax", String(24 * 60));
  });

  describe("never renders a minutes digit >= 60 (rounding-carry regression)", () => {
    // Cases close to the bottom of each hour: the residual seconds round up
    // to 60 in the naive Math.round((ms - hours*HOUR) / 60_000) implementation.
    const carryCases = [
      { ms: 2 * HOUR + 59 * 60_000 + 30_000, expected: "3h left" },
      { ms: 1 * HOUR + 59 * 60_000 + 30_000, expected: "2h left" },
      { ms: 5 * HOUR + 59 * 60_000 + 45_000, expected: "6h left" },
      // 59m 30s should carry up to "1h left", not render as bare "60m left".
      { ms: 59 * 60_000 + 30_000, expected: "1h left" },
    ];

    for (const { ms, expected } of carryCases) {
      it(`renders ${expected} for ${ms}ms (no 60m carry overflow)`, async () => {
        render(<SLABar expiresAt={expiresIn(ms)} />);
        const bar = await screen.findByTestId("sla-bar");
        expect(bar).toHaveTextContent(expected);
        expect(bar.textContent ?? "").not.toMatch(/\b60m\b/);
      });
    }

    it("renders '0m left' when fewer than 30 seconds remain", async () => {
      render(<SLABar expiresAt={expiresIn(20 * 1000)} />);
      const bar = await screen.findByTestId("sla-bar");
      expect(bar).toHaveTextContent("0m left");
    });

    it("preserves non-boundary hours+minutes splits", async () => {
      render(<SLABar expiresAt={expiresIn(25 * HOUR + 5 * 60_000)} />);
      const bar = await screen.findByTestId("sla-bar");
      expect(bar).toHaveTextContent("25h 5m left");
    });

    it("over-SLA case carries cleanly (never 'over 60m')", async () => {
      // 59m 45s past deadline: naive Math.round would render "over 60m"
      // instead of carrying into "over 1h".
      render(<SLABar expiresAt={expiresIn(-(59 * 60_000 + 45_000))} />);
      const bar = await screen.findByTestId("sla-bar");
      expect(bar).toHaveTextContent("over 1h");
      expect(bar.textContent ?? "").not.toMatch(/over 60m/);
    });
  });
});
