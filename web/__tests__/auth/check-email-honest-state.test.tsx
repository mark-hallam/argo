import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Brief: launch-email-delivery-honesty-and-public-base-url.
// The check-email page reads the honest `?delivery=` status the gateway
// threads into the signup response and shows a truthful state instead of
// an unconditional "Check your inbox". A stranger whose email never sent
// must be told so, with a support path — never left waiting forever.

const searchParamsMock = vi.hoisted(() => ({
  current: new URLSearchParams(),
}));

vi.mock("next/navigation", () => ({
  useSearchParams: () => searchParamsMock.current,
}));

import CheckEmailPage from "@/app/auth/check-email/page";

beforeEach(() => {
  searchParamsMock.current = new URLSearchParams();
});

describe("CheckEmailPage — honest delivery states", () => {
  it("shows the happy 'Check your inbox' state when delivery=sent", () => {
    searchParamsMock.current = new URLSearchParams({
      email: "ada@example.com",
      delivery: "sent",
    });
    render(<CheckEmailPage />);
    expect(screen.getByText(/check your/i)).toBeInTheDocument();
    expect(screen.queryByTestId("check-email-honest")).not.toBeInTheDocument();
  });

  it("defaults to the happy state when no delivery param is present (older gateway)", () => {
    searchParamsMock.current = new URLSearchParams({ email: "ada@example.com" });
    render(<CheckEmailPage />);
    expect(screen.queryByTestId("check-email-honest")).not.toBeInTheDocument();
  });

  it("shows the 'email not set up' honest state when delivery=not_configured", () => {
    searchParamsMock.current = new URLSearchParams({
      email: "ada@example.com",
      delivery: "not_configured",
    });
    render(<CheckEmailPage />);
    const honest = screen.getByTestId("check-email-honest");
    expect(honest).toHaveAttribute("data-delivery", "not_configured");
    expect(honest.textContent).toMatch(/not set up/i);
    // Honest copy must not promise an email was sent, and must offer support.
    expect(screen.getByText(/email delivery isn't configured/i)).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: /contact support/i }),
    ).toHaveAttribute("href", "mailto:support@skycot.com");
  });

  it("shows the 'delivery failed' honest state when delivery=delivery_failed", () => {
    searchParamsMock.current = new URLSearchParams({
      email: "ada@example.com",
      delivery: "delivery_failed",
    });
    render(<CheckEmailPage />);
    const honest = screen.getByTestId("check-email-honest");
    expect(honest).toHaveAttribute("data-delivery", "delivery_failed");
    expect(honest.textContent).toMatch(/delivery failed/i);
    // No provider error detail is ever leaked — only the coarse status.
    expect(screen.getByText(/something went wrong sending/i)).toBeInTheDocument();
    expect(screen.getByTestId("check-email-resend")).toBeInTheDocument();
  });
});
