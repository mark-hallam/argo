import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

let mockPathname = "/landing";

vi.mock("next/navigation", () => ({
  usePathname: () => mockPathname,
}));

import { MarketingNav } from "@/components/marketing/MarketingNav";

beforeEach(() => {
  mockPathname = "/landing";
});

describe("MarketingNav — desktop", () => {
  it("renders the SKYCOT wordmark + canonical link row + sign-in + CTA", () => {
    render(<MarketingNav />);
    expect(screen.getAllByText("SKYCOT")[0]).toBeInTheDocument();
    const desktopNav = screen.getByRole("navigation", { name: "Marketing" });
    const navLinks = within(desktopNav).getAllByRole("link");
    const labels = navLinks.map((a) => a.textContent);
    expect(labels).toEqual([
      "How it works",
      "Features",
      "Compare",
      "Roster",
      "Pricing",
      "Docs",
      "Changelog",
    ]);
    expect(screen.getByRole("link", { name: /sign in/i })).toBeInTheDocument();
    expect(
      screen.getAllByRole("link", { name: /hire your first teammate/i })[0],
    ).toBeInTheDocument();
  });

  it("marks the active page via aria-current when the pathname matches a nav target", () => {
    mockPathname = "/pricing";
    render(<MarketingNav />);
    const desktopNav = screen.getByRole("navigation", { name: "Marketing" });
    const pricing = within(desktopNav).getByRole("link", { name: "Pricing" });
    expect(pricing).toHaveAttribute("aria-current", "page");
    const compare = within(desktopNav).getByRole("link", { name: "Compare" });
    expect(compare).not.toHaveAttribute("aria-current");
  });
});

describe("MarketingNav — mobile sheet", () => {
  it("opens the slide-out sheet when the hamburger is clicked", async () => {
    const user = userEvent.setup();
    render(<MarketingNav />);
    await user.click(screen.getByTestId("marketing-nav-open"));
    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    const dialog = screen.getByRole("dialog");
    expect(
      within(dialog).getByRole("link", { name: "How it works" }),
    ).toBeInTheDocument();
    expect(
      within(dialog).getByRole("link", { name: /hire your first teammate/i }),
    ).toBeInTheDocument();
  });

  it("closes the sheet when Esc is pressed", async () => {
    const user = userEvent.setup();
    render(<MarketingNav />);
    await user.click(screen.getByTestId("marketing-nav-open"));
    await screen.findByRole("dialog");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("autofocuses the first sheet link on open", async () => {
    const user = userEvent.setup();
    render(<MarketingNav />);
    await user.click(screen.getByTestId("marketing-nav-open"));
    const dialog = await screen.findByRole("dialog");
    const first = within(dialog).getByRole("link", { name: "How it works" });
    await waitFor(() => expect(first).toHaveFocus());
  });

  it("renders extras (e.g. RSS) in both desktop and mobile sheet", async () => {
    const user = userEvent.setup();
    render(<MarketingNav extras={<a href="/feed.xml">RSS</a>} />);
    expect(screen.getAllByRole("link", { name: "RSS" })[0]).toBeInTheDocument();
    await user.click(screen.getByTestId("marketing-nav-open"));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByRole("link", { name: "RSS" })).toBeInTheDocument();
  });
});
