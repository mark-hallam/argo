import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DutiesCard } from "./DutiesCard";
import type { WorkerDuty } from "@/lib/api/worker-overview";

/**
 * Day-1 unblock FIX 4 — honest schedule labelling.
 *
 * The schedules card shows enabled toggles while the platform-wide
 * autofire flag is OFF server-side, so nothing ever fires on its own.
 * The SPA can't read that flag, so the card carries a static, muted
 * one-line hint telling the operator schedules run once the scheduler
 * is armed and pointing them at Run now (the path that works today).
 */

const DUTIES: WorkerDuty[] = [
  { id: "d1", label: "Screen inbound applicants", schedule: "weekdays 09:00", enabled: true },
  { id: "d2", label: "Send weekly summary", schedule: "daily 06:30", enabled: false },
];

describe("DutiesCard — scheduler-armed hint", () => {
  it("renders the honest hint under a non-empty schedules list", () => {
    render(<DutiesCard duties={DUTIES} canEdit={false} />);

    const hint = screen.getByTestId("duties-autofire-hint");
    expect(hint.textContent).toMatch(/scheduler\s+is armed/i);
    expect(hint.textContent).toMatch(/use Run now/i);
    // Muted, quiet styling — not a banner.
    expect(hint.className).toContain("text-xs");
    expect(hint.className).toContain("--muted-foreground");
  });

  it("does not render the hint when there are no schedules", () => {
    render(<DutiesCard duties={[]} canEdit={false} />);

    expect(screen.queryByTestId("duties-autofire-hint")).toBeNull();
    expect(screen.getByText(/No schedules yet/i)).toBeInTheDocument();
  });

  it("does not render the hint in the error state", () => {
    render(
      <DutiesCard duties={DUTIES} canEdit={false} error="Couldn't load schedules." />,
    );

    expect(screen.queryByTestId("duties-autofire-hint")).toBeNull();
  });
});
