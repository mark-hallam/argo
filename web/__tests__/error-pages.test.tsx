import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ComponentType } from "react";

import NotFound from "@/app/not-found";
import MaintenancePage from "@/app/maintenance/page";
import GlobalError from "@/app/error";
import WorkerDetailError from "@/app/(app)/workers/[workerId]/error";
import AuditError from "@/app/(app)/audit/error";
import AuditActionError from "@/app/(app)/audit/action/[actionId]/error";
import AuditReplayError from "@/app/(app)/audit/action/[actionId]/replay/error";
import OnboardingError from "@/app/(setup)/onboarding/error";
import HireError from "@/app/(hire)/hire/error";
import AuthError from "@/app/auth/error";
import CohortsError from "@/app/(app)/cohorts/error";
import SkillsError from "@/app/(app)/skills/error";
import SettingsError from "@/app/(app)/settings/error";

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

describe("route-segment error boundaries", () => {
  type SegmentError = ComponentType<{
    error: Error & { digest?: string };
    reset: () => void;
  }>;

  const segments: Array<{ name: string; Component: SegmentError; logTag: string }> = [
    { name: "/workers/[workerId]", Component: WorkerDetailError, logTag: "workers/[workerId]" },
    { name: "/audit", Component: AuditError, logTag: "audit" },
    { name: "/audit/action/[actionId]", Component: AuditActionError, logTag: "audit/action/[actionId]" },
    { name: "/audit/action/[actionId]/replay", Component: AuditReplayError, logTag: "audit/action/[actionId]/replay" },
    { name: "/onboarding", Component: OnboardingError, logTag: "onboarding" },
    { name: "/hire", Component: HireError, logTag: "hire" },
    { name: "/auth", Component: AuthError, logTag: "auth" },
    { name: "/cohorts", Component: CohortsError, logTag: "cohorts" },
    { name: "/skills", Component: SkillsError, logTag: "skills" },
    { name: "/settings", Component: SettingsError, logTag: "settings" },
  ];

  for (const { name, Component, logTag } of segments) {
    it(`${name} renders ErrorShell with reset action and logs caught error`, () => {
      const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
      const reset = vi.fn();
      const error = new Error(`boom in ${name}`);

      render(<Component error={error} reset={reset} />);

      expect(screen.getByTestId("error-shell")).toBeInTheDocument();
      expect(screen.getByRole("main")).toBeInTheDocument();
      expect(screen.getByRole("heading", { level: 1 })).toBeInTheDocument();
      const resetButton = screen.getByRole("button");
      resetButton.click();
      expect(reset).toHaveBeenCalledTimes(1);

      const loggedWithTag = consoleErrorSpy.mock.calls.some((call) =>
        typeof call[0] === "string" && call[0].includes(logTag),
      );
      expect(loggedWithTag).toBe(true);

      consoleErrorSpy.mockRestore();
    });
  }
});
