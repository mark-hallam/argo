import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// `SsoButtons` is INTENTIONALLY gated off until the gateway OAuth routes
// AND the provider OAuth clients are configured (see the component's own
// docstring): with `NEXT_PUBLIC_SSO_ENABLED !== "1"` it returns null so a
// click can't dump the user on a raw gateway 404. That product gate is
// CORRECT, so this test pins BOTH states rather than forcing the buttons
// to render. The gate is read at import time, so each case re-imports the
// module after stubbing the env var. This test does NOT enable SSO in the
// product — flipping the flag on is an operator/product decision.

const ENV_KEY = "NEXT_PUBLIC_SSO_ENABLED";

async function importFresh(enabled: boolean) {
  vi.resetModules();
  vi.stubEnv(ENV_KEY, enabled ? "1" : "");
  return import("./SsoButtons");
}

describe("SsoButtons (intentional gate)", () => {
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
    vi.unstubAllEnvs();
    Object.defineProperty(window, "location", {
      configurable: true,
      value: originalLocation,
    });
  });

  it("renders nothing while SSO is gated off (NEXT_PUBLIC_SSO_ENABLED !== '1')", async () => {
    const { SsoButtons } = await importFresh(false);
    const { container } = render(<SsoButtons />);
    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByTestId("sso-row")).toBeNull();
    expect(screen.queryByTestId("sso-button-google")).toBeNull();
  });

  it("renders Google, GitHub, and Microsoft buttons once enabled", async () => {
    const { SsoButtons } = await importFresh(true);
    render(<SsoButtons />);
    expect(screen.getByTestId("sso-button-google")).toBeTruthy();
    expect(screen.getByTestId("sso-button-github")).toBeTruthy();
    expect(screen.getByTestId("sso-button-microsoft")).toBeTruthy();
  });

  it("navigates to the OAuth start URL on click once enabled", async () => {
    vi.stubEnv("NEXT_PUBLIC_GATEWAY_URL", "http://gw.test");
    const { SsoButtons } = await importFresh(true);
    const user = userEvent.setup();
    render(<SsoButtons redirectTo="/dashboard" />);
    await user.click(screen.getByTestId("sso-button-google"));
    expect(window.location.href).toContain(
      "/api/v1/auth/oauth/google/start?redirect_to=%2Fdashboard",
    );
  });

  it("applies the highlight class to the matched provider once enabled", async () => {
    const { SsoButtons } = await importFresh(true);
    render(<SsoButtons highlight="microsoft" />);
    const button = screen.getByTestId("sso-button-microsoft");
    expect(button.className).toContain("sso-btn-highlight");
    const other = screen.getByTestId("sso-button-google");
    expect(other.className).not.toContain("sso-btn-highlight");
  });
});
