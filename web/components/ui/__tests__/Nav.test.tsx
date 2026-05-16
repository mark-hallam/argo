import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Nav } from "../Nav";

describe("Nav", () => {
  it("renders brand, links, and the burger toggles a mobile panel", async () => {
    const user = userEvent.setup();
    render(
      <Nav
        brand="skycot"
        links={[
          { label: "Features", href: "/features" },
          { label: "Pricing", href: "/pricing" },
        ]}
        cta={<button>Sign in</button>}
      />,
    );
    expect(screen.getByText("skycot")).toBeInTheDocument();
    // Desktop link list (hidden on mobile via CSS, still in DOM)
    expect(screen.getAllByRole("link", { name: "Features" }).length).toBeGreaterThan(0);

    const burger = screen.getByTestId("nav-burger");
    expect(burger).toHaveAttribute("aria-expanded", "false");
    await user.click(burger);
    expect(burger).toHaveAttribute("aria-expanded", "true");
  });
});
