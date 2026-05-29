import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ThemeProvider as NextThemesProvider } from "next-themes";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

let mockPathname = "/landing";

vi.mock("next/navigation", () => ({
  usePathname: () => mockPathname,
}));

import { MarketingNav } from "@/components/marketing/MarketingNav";

// ThemeToggle (rendered inside MarketingNav per whole-site F07 fix) calls
// useTheme() from next-themes, so the test tree needs a NextThemesProvider
// ancestor. We wrap with the raw next-themes provider rather than the
// SKYCOT ThemeProvider to keep this test focused on the nav surface.
function renderNav(node: ReactNode) {
  return render(<NextThemesProvider attribute="class">{node}</NextThemesProvider>);
}

beforeEach(() => {
  mockPathname = "/landing";
  try {
    window.localStorage.clear();
  } catch {
    /* localStorage may be unavailable */
  }
  document.cookie = "skycot-color-mode=; Path=/; Max-Age=0";
});

describe("MarketingNav — desktop", () => {
  it("renders the SKYCOT wordmark + canonical link row + sign-in + CTA", () => {
    renderNav(<MarketingNav />);
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
    renderNav(<MarketingNav />);
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
    renderNav(<MarketingNav />);
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

  it("wires a stable aria-controls id matching the dialog content id", async () => {
    // Regression: Radix's auto-generated useId() value drifted between
    // SSR and client, flagging an aria-controls hydration mismatch on the
    // burger button (the red "1 Issue" dev badge on every marketing page).
    // Both sides are now pinned to a constant so SSR === client.
    const user = userEvent.setup();
    renderNav(<MarketingNav />);
    const burger = screen.getByTestId("marketing-nav-open");
    expect(burger).toHaveAttribute("aria-controls", "mk-mobile-menu");
    await user.click(burger);
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveAttribute("id", "mk-mobile-menu");
  });

  it("closes the sheet when Esc is pressed", async () => {
    const user = userEvent.setup();
    renderNav(<MarketingNav />);
    await user.click(screen.getByTestId("marketing-nav-open"));
    await screen.findByRole("dialog");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("autofocuses the first sheet link on open", async () => {
    const user = userEvent.setup();
    renderNav(<MarketingNav />);
    await user.click(screen.getByTestId("marketing-nav-open"));
    const dialog = await screen.findByRole("dialog");
    const first = within(dialog).getByRole("link", { name: "How it works" });
    await waitFor(() => expect(first).toHaveFocus());
  });

  it("renders extras (e.g. RSS) in both desktop and mobile sheet", async () => {
    const user = userEvent.setup();
    renderNav(<MarketingNav extras={<a href="/feed.xml">RSS</a>} />);
    expect(screen.getAllByRole("link", { name: "RSS" })[0]).toBeInTheDocument();
    await user.click(screen.getByTestId("marketing-nav-open"));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByRole("link", { name: "RSS" })).toBeInTheDocument();
  });
});

describe("MarketingNav — theme toggle (whole-site F07)", () => {
  it("renders a theme toggle in the desktop top-right strip", () => {
    renderNav(<MarketingNav />);
    const toggles = screen.getAllByTestId("appshell-theme-toggle");
    expect(toggles.length).toBeGreaterThanOrEqual(1);
    const toggle = toggles[0];
    expect(toggle).toHaveAttribute(
      "aria-label",
      expect.stringMatching(/switch to (light|dark) theme/i),
    );
  });

  it("writes the color-mode cookie when clicked", async () => {
    const user = userEvent.setup();
    renderNav(<MarketingNav />);
    const toggle = screen.getAllByTestId("appshell-theme-toggle")[0];
    await user.click(toggle);
    await waitFor(() => {
      expect(document.cookie).toMatch(/skycot-color-mode=(light|dark)/);
    });
  });

  it("also surfaces the theme toggle inside the mobile sheet", async () => {
    const user = userEvent.setup();
    renderNav(<MarketingNav />);
    await user.click(screen.getByTestId("marketing-nav-open"));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByTestId("appshell-theme-toggle")).toBeInTheDocument();
  });
});
