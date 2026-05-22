import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import ApprovalsLandingPage from "@/app/(app)/approvals/page";

describe("/approvals landing page", () => {
  it("renders the inbox empty-state with both CTAs", () => {
    render(<ApprovalsLandingPage />);

    expect(screen.getByTestId("approvals-landing")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 1, name: /approvals/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /no approvals waiting/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: /open the inbox/i }),
    ).toHaveAttribute("href", "/");
    expect(
      screen.getByRole("link", { name: /see your workers/i }),
    ).toHaveAttribute("href", "/workers");
  });

  it("mounts without throwing on a fresh workspace (no providers required)", () => {
    expect(() => render(<ApprovalsLandingPage />)).not.toThrow();
  });
});
