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
});
