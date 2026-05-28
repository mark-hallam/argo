/**
 * Pins the demo-live suppression behavior added in
 * `fix-team-page-mock-bypass-pill-visible-to-camera-in-demo-live` —
 * scene-1-and-2 F10 of the 2026-05-28-0252 UX audit.
 *
 * When `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` the chip must not render in
 * any data-source branch, so the v1.0 demo MP4 doesn't catch a stray
 * "MOCK (AUTH-BYPASS)" pill on camera. In normal dev (demo-live off)
 * the chip still renders for `mock-auth-bypass` and `live`, and
 * stays hidden for `mock-offline` per the 2026-05-20 prior fix.
 */
import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { DataSourceChip } from "./DataSourceChip";

const ORIGINAL_DEMO_LIVE = process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE;

function setDemoLive(on: boolean) {
  if (on) process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
  else delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE;
}

beforeEach(() => setDemoLive(false));

afterEach(() => {
  if (ORIGINAL_DEMO_LIVE === undefined) {
    delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE;
  } else {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = ORIGINAL_DEMO_LIVE;
  }
});

describe("DataSourceChip", () => {
  it("renders the mock-auth-bypass pill in normal dev", () => {
    setDemoLive(false);
    render(<DataSourceChip source="mock-auth-bypass" />);
    expect(screen.getByTestId("data-source-chip")).toBeInTheDocument();
  });

  it("suppresses the mock-auth-bypass pill when demo-live is on", () => {
    setDemoLive(true);
    render(<DataSourceChip source="mock-auth-bypass" />);
    expect(screen.queryByTestId("data-source-chip")).not.toBeInTheDocument();
  });

  it("suppresses the live pill when demo-live is on", () => {
    setDemoLive(true);
    render(<DataSourceChip source="live" />);
    expect(screen.queryByTestId("data-source-chip")).not.toBeInTheDocument();
  });

  it("keeps mock-offline hidden in normal dev (existing behavior)", () => {
    setDemoLive(false);
    render(<DataSourceChip source="mock-offline" />);
    expect(screen.queryByTestId("data-source-chip")).not.toBeInTheDocument();
  });

  it("renders nothing when source is undefined", () => {
    setDemoLive(false);
    const { container } = render(<DataSourceChip source={undefined} />);
    expect(container).toBeEmptyDOMElement();
  });
});
