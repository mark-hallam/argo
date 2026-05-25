import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const ssoApi = vi.hoisted(() => ({
  getSamlConfig: vi.fn(),
  samlMetadataUrl: vi.fn(() => "http://gw.test/api/v1/auth/saml/metadata"),
  saveSamlConfig: vi.fn(),
  testSamlConnection: vi.fn(),
}));

vi.mock("@/lib/api/sso", () => ssoApi);

import SecuritySettingsPage from "./page";

describe("settings security SAML panel", () => {
  beforeEach(() => {
    ssoApi.getSamlConfig.mockResolvedValue({ configured: false, config: null });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders a neutral fallback when the SAML status request fails", async () => {
    ssoApi.getSamlConfig.mockRejectedValue(new TypeError("Failed to fetch"));

    render(<SecuritySettingsPage />);

    await waitFor(() => {
      expect(screen.getByTestId("saml-status-pill")).toHaveTextContent("Not configured");
    });
    expect(screen.getByTestId("saml-fallback")).toHaveTextContent(
      "Not configured · Set up SAML SSO to connect your identity provider.",
    );
    expect(screen.queryByText("Failed to fetch")).not.toBeInTheDocument();
    expect(screen.queryByTestId("saml-error")).not.toBeInTheDocument();
  });

  it("renders the not-configured state with the configure CTA", async () => {
    ssoApi.getSamlConfig.mockResolvedValue({ configured: false, config: null });

    render(<SecuritySettingsPage />);

    await waitFor(() => {
      expect(screen.getByTestId("saml-status-pill")).toHaveTextContent("Not configured");
    });
    expect(screen.getByTestId("saml-fallback")).toHaveTextContent("Set up SAML SSO");
    expect(screen.getByTestId("saml-configure")).toHaveTextContent("Configure SAML");
  });

  it("renders the configured state summary", async () => {
    ssoApi.getSamlConfig.mockResolvedValue({
      configured: true,
      config: {
        idpEntityId: "http://www.okta.com/abc",
        ssoUrl: "https://example.okta.com/sso/saml",
        certificate: "-----BEGIN CERTIFICATE-----\nMIID0123456789abcdef\n-----END CERTIFICATE-----",
        flavor: "okta",
        allowOwnerBypass: true,
      },
    });

    render(<SecuritySettingsPage />);

    await waitFor(() => {
      expect(screen.getByTestId("saml-status-pill")).toHaveTextContent("Configured: Okta");
    });
    expect(screen.getByTestId("saml-summary")).toHaveTextContent("http://www.okta.com/abc");
    expect(screen.queryByTestId("saml-fallback")).not.toBeInTheDocument();
    expect(screen.queryByText("Failed to fetch")).not.toBeInTheDocument();
  });
});
