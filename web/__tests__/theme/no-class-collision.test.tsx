import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ThemeProvider } from "@/components/skycot/ThemeProvider";
import { ThemeToggle } from "@/components/skycot/ThemeToggle";

/**
 * Regression: evaluator round-6 (2026-05-23) Chrome MCP run reported
 * `<html>` carrying BOTH `light` AND `dark` classes at the same time.
 * The fix is to pin SSR to `light` and let next-themes drive a single
 * resolved class on the client. These tests assert that the SPA
 * ThemeProvider never lands in the collided state and that the
 * ThemeToggle flips the class deterministically.
 */

function classes() {
  return Array.from(document.documentElement.classList);
}

function themeClasses() {
  return classes().filter((c) => c === "light" || c === "dark");
}

function resetHtmlClass() {
  document.documentElement.className = "";
  document.documentElement.removeAttribute("data-theme");
}

beforeEach(() => {
  resetHtmlClass();
  try {
    window.localStorage.clear();
  } catch {
    /* localStorage may be unavailable */
  }
});

afterEach(() => {
  resetHtmlClass();
});

describe("html theme class collision (regression for evaluator round-6)", () => {
  it("ThemeProvider mounts with exactly one of light/dark on <html>", async () => {
    render(
      <ThemeProvider>
        <div data-testid="theme-child">child</div>
      </ThemeProvider>,
    );

    expect(screen.getByTestId("theme-child")).toBeInTheDocument();

    // Wait one microtask tick so next-themes' effect has applied the
    // initial class. The default ("light") must have landed exactly once.
    await act(async () => {
      await Promise.resolve();
    });

    const themed = themeClasses();
    expect(themed.length).toBe(1);
    expect(themed[0]).toBe("light");
  });

  it("ThemeToggle flips the class on click without producing a collision", async () => {
    const user = userEvent.setup();

    render(
      <ThemeProvider>
        <ThemeToggle />
      </ThemeProvider>,
    );

    await act(async () => {
      await Promise.resolve();
    });

    const toggle = screen.getByTestId("appshell-theme-toggle");
    expect(toggle).toBeInTheDocument();
    expect(themeClasses()).toEqual(["light"]);

    await user.click(toggle);

    await act(async () => {
      await Promise.resolve();
    });

    const themed = themeClasses();
    expect(themed.length).toBe(1);
    expect(themed[0]).toBe("dark");

    await user.click(toggle);

    await act(async () => {
      await Promise.resolve();
    });

    expect(themeClasses()).toEqual(["light"]);
  });

  it("ThemeToggle is keyboard-accessible (Tab + Enter)", async () => {
    const user = userEvent.setup();

    render(
      <ThemeProvider>
        <ThemeToggle />
      </ThemeProvider>,
    );

    await act(async () => {
      await Promise.resolve();
    });

    await user.tab();
    const toggle = screen.getByTestId("appshell-theme-toggle");
    expect(toggle).toHaveFocus();
    expect(toggle).toHaveAttribute("aria-label", expect.stringMatching(/dark theme/i));

    await user.keyboard("{Enter}");

    await act(async () => {
      await Promise.resolve();
    });

    expect(themeClasses()).toEqual(["dark"]);
    expect(toggle).toHaveAttribute("aria-label", expect.stringMatching(/light theme/i));
  });
});
