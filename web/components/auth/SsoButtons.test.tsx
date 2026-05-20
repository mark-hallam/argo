import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SsoButtons } from "./SsoButtons";

describe("SsoButtons", () => {
  const originalLocation = window.location;
  let assignSpy: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    assignSpy = vi.fn();
    Object.defineProperty(window, "location", {
      configurable: true,
      value: { ...originalLocation, href: "", assign: assignSpy },
    });
  });

  afterEach(() => {
    Object.defineProperty(window, "location", {
      configurable: true,
      value: originalLocation,
    });
  });

  it("renders Google, GitHub, and Microsoft buttons", () => {
    render(<SsoButtons />);
    expect(screen.getByTestId("sso-button-google")).toBeTruthy();
    expect(screen.getByTestId("sso-button-github")).toBeTruthy();
    expect(screen.getByTestId("sso-button-microsoft")).toBeTruthy();
  });

  it("navigates to the OAuth start URL on click", async () => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    const user = userEvent.setup();
    render(<SsoButtons redirectTo="/dashboard" />);
    await user.click(screen.getByTestId("sso-button-google"));
    expect(window.location.href).toContain(
      "/api/v1/auth/oauth/google/start?redirect_to=%2Fdashboard",
    );
  });

  it("applies the highlight class to the matched provider", () => {
    render(<SsoButtons highlight="microsoft" />);
    const button = screen.getByTestId("sso-button-microsoft");
    expect(button.className).toContain("sso-btn-highlight");
    const other = screen.getByTestId("sso-button-google");
    expect(other.className).not.toContain("sso-btn-highlight");
  });
});
