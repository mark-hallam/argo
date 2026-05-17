import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import NotFound from "@/app/not-found";
import MaintenancePage from "@/app/maintenance/page";
import GlobalError from "@/app/error";

describe("error pages", () => {
  it("404 page renders semantic landmarks + suggested links", () => {
    render(<NotFound />);
    expect(screen.getByRole("main")).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(/that page isn't here/i);
    expect(screen.getByRole("link", { name: /approvals/i })).toHaveAttribute("href", "/");
    expect(screen.getByRole("link", { name: /workers/i })).toHaveAttribute("href", "/workers");
    expect(screen.getByRole("link", { name: /docs/i })).toHaveAttribute("href", "/docs");
  });

  it("500 error boundary renders reset, status, and support actions", () => {
    const reset = vi.fn();
    render(<GlobalError error={new Error("boom")} reset={reset} />);
    expect(screen.getByRole("main")).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(/something went wrong/i);
    expect(screen.getByRole("button", { name: /try again/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /view status/i })).toHaveAttribute(
      "href",
      "https://status.skycot.com",
    );
    expect(screen.getByRole("link", { name: /contact support/i })).toHaveAttribute(
      "href",
      "mailto:support@skycot.com",
    );
  });

  it("maintenance page renders status link", () => {
    render(<MaintenancePage />);
    expect(screen.getByRole("main")).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(/we'll be right back/i);
    expect(screen.getByRole("link", { name: /live status/i })).toHaveAttribute(
      "href",
      "https://status.skycot.com",
    );
  });
});
