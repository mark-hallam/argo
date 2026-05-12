import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { EXAMPLE_ROLES } from "@/lib/hire/examples";
import { TeamEmptyState } from "./TeamEmptyState";

/**
 * `<TeamEmptyState>` is a presentational shell with `<Link>` anchors
 * — no router state is exercised, so we only need to assert that the
 * rendered `href` attributes carry the expected `?example=<id>`
 * payload. `next/navigation` is mocked to keep `useSearchParams`
 * happy in case `next/link` reaches for it during prefetch heuristics.
 */
vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  usePathname: () => "/team",
  useSearchParams: () => new URLSearchParams(),
}));

describe("TeamEmptyState", () => {
  it("renders the locked heading and sub-line copy", () => {
    render(<TeamEmptyState />);
    expect(
      screen.getByRole("heading", { level: 2, name: /hire your first ai teammate\./i }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        /pick a starting point — you can adjust everything in the next minute\./i,
      ),
    ).toBeInTheDocument();
  });

  it("renders exactly one tile per example role with id-derived testid", () => {
    render(<TeamEmptyState />);
    const grid = screen.getByTestId("team-empty-state-tiles");
    const tiles = within(grid).getAllByRole("link");
    expect(tiles).toHaveLength(EXAMPLE_ROLES.length);
    for (const role of EXAMPLE_ROLES) {
      const tile = screen.getByTestId(`team-example-tile-${role.id}`);
      expect(tile).toHaveAttribute("href", `/hire?example=${role.id}`);
      expect(within(tile).getByText(role.title)).toBeInTheDocument();
      expect(within(tile).getByText(role.description)).toBeInTheDocument();
    }
  });

  it("each tile carries a focusable 'Start with this' affordance", () => {
    render(<TeamEmptyState />);
    for (const role of EXAMPLE_ROLES) {
      const cta = screen.getByTestId(`team-example-tile-cta-${role.id}`);
      expect(cta).toHaveTextContent(/start with this/i);
    }
  });

  it("the secondary 'Or describe your own' link routes to /hire with no query", () => {
    render(<TeamEmptyState />);
    const fallback = screen.getByTestId("team-empty-state-describe-own");
    expect(fallback).toHaveAttribute("href", "/hire");
    expect(fallback).toHaveTextContent(/or describe your own/i);
  });

  it("the feature-list disclosure toggles open and surfaces the five locked bullets", async () => {
    const user = userEvent.setup();
    render(<TeamEmptyState />);
    const disclosure = screen.getByTestId("team-empty-state-disclosure") as HTMLDetailsElement;
    expect(disclosure.open).toBe(false);
    await user.click(screen.getByTestId("team-empty-state-disclosure-summary"));
    expect(disclosure.open).toBe(true);
    const list = screen.getByTestId("team-empty-state-feature-list");
    const items = within(list).getAllByRole("listitem");
    expect(items).toHaveLength(5);
    expect(within(list).getByText(/voice profile inferred from how you write/i)).toBeInTheDocument();
    expect(within(list).getByText(/approvals via slack, imessage, sms, email/i)).toBeInTheDocument();
    expect(within(list).getByText(/learns from your corrections/i)).toBeInTheDocument();
    expect(within(list).getByText(/audit trail for every action/i)).toBeInTheDocument();
    expect(within(list).getByText(/pause or adjust her anytime/i)).toBeInTheDocument();
  });

  it("only renders one h2 — tile titles are h3 so the empty-state heading remains the page sub-section anchor", () => {
    render(<TeamEmptyState />);
    expect(screen.getAllByRole("heading", { level: 2 })).toHaveLength(1);
    expect(screen.getAllByRole("heading", { level: 3 })).toHaveLength(EXAMPLE_ROLES.length);
  });
});
