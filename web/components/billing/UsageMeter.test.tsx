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

describe("UsageMeter", () => {
  it("renders the tier and a progress bar from live usage data", () => {
    render(<UsageMeter usage={usage({ used: 40, limit: 100 })} />);
    expect(screen.getByText("Free")).toBeInTheDocument();
    expect(screen.getByText(/Used 40 of 100 actions this month/)).toBeInTheDocument();
    const bar = screen.getByRole("progressbar", { name: /Monthly action usage/ });
    // 40 / 100 = 40%
    expect(bar).toHaveAttribute("aria-valuenow", "40");
  });

  it("crosses into the warning tone at 80% of cap", () => {
    render(<UsageMeter usage={usage({ used: 4000, limit: 5000 })} />);
    const bar = screen.getByRole("progressbar", { name: /Monthly action usage/ });
    expect(bar).toHaveAttribute("aria-valuenow", "80");
    expect(bar).toHaveAttribute("data-tone", "warning");
  });

  it("shows an honest zero state (empty bar, real zero count, never fabricated)", () => {
    render(<UsageMeter usage={usage({ used: 0, limit: 100 })} />);
    expect(screen.getByText(/Used 0 of 100 actions this month/)).toBeInTheDocument();
    const bar = screen.getByRole("progressbar", { name: /Monthly action usage/ });
    expect(bar).toHaveAttribute("aria-valuenow", "0");
  });

  it("renders the unlimited tier without a bar (no percentage of an unlimited cap)", () => {
    render(
      <UsageMeter
        usage={usage({ tier: "enterprise", display_name: "Enterprise", used: 1200, limit: 0, remaining: null })}
      />,
    );
    expect(screen.getByText("Enterprise")).toBeInTheDocument();
    expect(screen.getByText(/Unlimited actions on this plan/)).toBeInTheDocument();
    expect(screen.getByText(/Used 1,200 of Unlimited actions this month/)).toBeInTheDocument();
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });
});
