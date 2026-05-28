import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

vi.mock("next/navigation", () => ({
  usePathname: () => "/landing",
}));

import { MarketingNav } from "@/components/marketing/MarketingNav";

/**
 * Originally pinned the Roster link at /team (from the
 * `fix-marketing-nav-add-team-link-and-customer-page-robots` brief).
 * Superseded by `fix-landing-roster-unauth-fallback-not-team-shell`:
 * /team is an in-app shell route and was a brittle landing target
 * for logged-out marketing visitors. The Roster link now points at
 * /teammates — a public marketing-mirror that renders inside the
 * marketing layout with no in-app chrome.
 */
describe("MarketingNav Roster link", () => {
  it("points at /teammates so logged-out investors land on a public roster, not the in-app shell", () => {
    render(<MarketingNav />);
    const desktopNav = screen.getByRole("navigation", { name: "Marketing" });
    const roster = within(desktopNav).getByRole("link", { name: "Roster" });
    expect(roster).toHaveAttribute("href", "/teammates");
  });
});
