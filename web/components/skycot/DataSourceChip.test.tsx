/**
 * Pins the opt-in chip behavior added in
 * `fix-team-page-mock-offline-chip-suppress-during-recording` — F7 of
 * the 2026-05-26-2007 chrome-live-walk-and-playwright UX audit.
 *
 * The chip is hidden by default in every build. Set
 * `NEXT_PUBLIC_SKYCOT_SHOW_DATA_SOURCE_CHIP=1` to opt back in for dev
 * use. The underlying source value stays inspectable via the
 * page-header wrapper's `data-source` attribute regardless.
 */
import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { DataSourceChip } from "./DataSourceChip";

const ORIGINAL_SHOW = process.env.NEXT_PUBLIC_SKYCOT_SHOW_DATA_SOURCE_CHIP;

function setShow(on: boolean) {
  if (on) process.env.NEXT_PUBLIC_SKYCOT_SHOW_DATA_SOURCE_CHIP = "1";
  else delete process.env.NEXT_PUBLIC_SKYCOT_SHOW_DATA_SOURCE_CHIP;
}

beforeEach(() => setShow(false));

afterEach(() => {
  if (ORIGINAL_SHOW === undefined) {
    delete process.env.NEXT_PUBLIC_SKYCOT_SHOW_DATA_SOURCE_CHIP;
  } else {
    process.env.NEXT_PUBLIC_SKYCOT_SHOW_DATA_SOURCE_CHIP = ORIGINAL_SHOW;
  }
});

describe("DataSourceChip", () => {
  it("is hidden by default for every source", () => {
    for (const source of ["live", "mock-offline", "mock-auth-bypass"] as const) {
      const { unmount } = render(<DataSourceChip source={source} />);
      expect(screen.queryByTestId("data-source-chip")).not.toBeInTheDocument();
      unmount();
    }
  });

  it("renders when the opt-in flag is set", () => {
    setShow(true);
    render(<DataSourceChip source="mock-auth-bypass" />);
    const chip = screen.getByTestId("data-source-chip");
    expect(chip).toBeInTheDocument();
    expect(chip).toHaveAttribute("data-source", "mock-auth-bypass");
  });

  it("renders nothing when source is undefined even with the flag set", () => {
    setShow(true);
    const { container } = render(<DataSourceChip source={undefined} />);
    expect(container).toBeEmptyDOMElement();
  });
});
