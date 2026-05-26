import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

vi.mock("next/navigation", () => ({
  usePathname: () => "/landing",
}));

import { MarketingNav } from "@/components/marketing/MarketingNav";

describe("MarketingNav includes /team", () => {
  it("renders a /team link in the desktop nav so investors can find the roster from any marketing page", () => {
    render(<MarketingNav />);
    const desktopNav = screen.getByRole("navigation", { name: "Marketing" });
    const roster = within(desktopNav).getByRole("link", { name: "Roster" });
    expect(roster).toHaveAttribute("href", "/team");
  });
});
