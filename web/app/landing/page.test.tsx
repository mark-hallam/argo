import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import MarketingLandingPage from "./page";

vi.mock("next/navigation", () => ({
  usePathname: () => "/landing",
}));

describe("MarketingLandingPage", () => {
  it("does not market multi-provider routing as a v1 hero capability", () => {
    render(<MarketingLandingPage />);

    expect(
      screen.getByRole("heading", { level: 1, name: /hire a teammate/i }),
    ).toBeInTheDocument();
    expect(document.body).not.toHaveTextContent(/multi-provider routing/i);
    expect(document.body).not.toHaveTextContent(/provider routing/i);
  });

  it("surfaces the four verified differentiators on the locked landing page", () => {
    render(<MarketingLandingPage />);

    expect(
      screen.getByText(/verified differentiators/i),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /Every action, provable/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /Truth-bounded claims/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /Approval-first in 9 channels/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /Persistent teammates/i })).toBeInTheDocument();
  });
});
