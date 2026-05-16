import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { LandingHero } from "../LandingHero";

describe("LandingHero", () => {
  it("renders headline, subcopy, eyebrow, and both CTAs", () => {
    render(
      <LandingHero
        eyebrow="Skycot"
        headline="Hire your AI team"
        subcopy="Operational AI for ambitious teams."
        primaryCta={<button>Get started</button>}
        secondaryCta={<button>Talk to us</button>}
      />,
    );
    expect(screen.getByRole("heading", { name: "Hire your AI team" })).toBeInTheDocument();
    expect(screen.getByText("Operational AI for ambitious teams.")).toBeInTheDocument();
    expect(screen.getByText("Skycot")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Get started" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Talk to us" })).toBeInTheDocument();
  });
});
