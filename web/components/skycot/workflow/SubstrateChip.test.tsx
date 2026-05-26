import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { WorkflowSubstrate } from "@/lib/api/workflows";
import { SubstrateChip, type SubstrateStatus } from "./SubstrateChip";

const SUBSTRATES: WorkflowSubstrate[] = [
  "macos",
  "linux",
  "windows",
  "e2b",
  "browser",
  "headless",
];

const STATUSES: SubstrateStatus[] = ["idle", "running", "completed", "failed"];

describe("SubstrateChip", () => {
  it("renders every substrate and status variant", () => {
    render(
      <div>
        {SUBSTRATES.flatMap((substrate) =>
          STATUSES.map((status) => (
            <SubstrateChip
              key={`${substrate}-${status}`}
              substrate={substrate}
              status={status}
              staticPulse
            />
          )),
        )}
      </div>,
    );

    const chips = screen.getAllByTestId("substrate-chip");
    expect(chips).toHaveLength(24);
    for (const substrate of SUBSTRATES) {
      for (const status of STATUSES) {
        expect(
          chips.some(
            (chip) =>
              chip.getAttribute("data-substrate") === substrate &&
              chip.getAttribute("data-status") === status,
          ),
        ).toBe(true);
      }
    }
  });

  it("marks the running variant for assistive tech", () => {
    render(<SubstrateChip substrate="macos" status="running" />);
    expect(screen.getByLabelText("macOS substrate running")).toBeInTheDocument();
  });
});
