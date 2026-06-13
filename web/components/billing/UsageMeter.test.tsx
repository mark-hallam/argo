import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { UsageMeter } from "./UsageMeter";
import type { UsageResponse } from "@/lib/api/usage";

function usage(overrides: Partial<UsageResponse> = {}): UsageResponse {
  return {
    tier: "free",
    display_name: "Free",
    used: 5,
    limit: 100,
    remaining: 95,
    resets_at: "2026-07-01T00:00:00Z",
    ...overrides,
  };
}

describe("UsageMeter (plan-identity header)", () => {
  it("renders the active tier name and reset date from live usage data", () => {
    render(<UsageMeter usage={usage({ display_name: "Free" })} />);
    expect(screen.getByText("Current plan")).toBeInTheDocument();
    expect(screen.getByText("Free")).toBeInTheDocument();
    expect(screen.getByText(/Resets /)).toBeInTheDocument();
  });

  it("renders the enterprise tier name", () => {
    render(
      <UsageMeter
        usage={usage({
          tier: "enterprise",
          display_name: "Enterprise",
          used: 1200,
          limit: 0,
          remaining: null,
        })}
      />,
    );
    expect(screen.getByText("Enterprise")).toBeInTheDocument();
  });

  it("does NOT render a second 'Actions this month' meter (de-dup: the meter lives only in MeterPanel)", () => {
    render(<UsageMeter usage={usage({ used: 40, limit: 100 })} />);
    // The canonical metered "Actions this month" row + bar now belong to
    // MeterPanel; this header must not duplicate them (shakedown re-test N2).
    expect(screen.queryByText(/Actions this month/)).not.toBeInTheDocument();
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
    expect(
      screen.queryByText(/actions this month/),
    ).not.toBeInTheDocument();
  });
});
