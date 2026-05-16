import { render, screen } from "@testing-library/react";
import { Github } from "lucide-react";
import { describe, expect, it } from "vitest";
import { Footer } from "../Footer";

describe("Footer", () => {
  it("renders sections, links, social icons, and copyright", () => {
    render(
      <Footer
        brand="skycot"
        tagline="Operational AI."
        sections={[
          { title: "Product", links: [{ label: "Features", href: "/features" }] },
          { title: "Docs", links: [{ label: "Quickstart", href: "/docs" }] },
        ]}
        social={[{ label: "GitHub", href: "https://github.com", icon: Github }]}
        copyright="© 2026 Skycot"
      />,
    );
    expect(screen.getByText("skycot")).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Product" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Features" })).toHaveAttribute("href", "/features");
    expect(screen.getByRole("link", { name: "GitHub" })).toBeInTheDocument();
    expect(screen.getByText("© 2026 Skycot")).toBeInTheDocument();
  });
});
