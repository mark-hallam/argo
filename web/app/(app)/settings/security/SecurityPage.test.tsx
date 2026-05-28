import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const ssoApi = vi.hoisted(() => ({
  getSamlConfig: vi.fn(),
  samlMetadataUrl: vi.fn(() => "http://gw.test/api/v1/auth/saml/metadata"),
  saveSamlConfig: vi.fn(),
  testSamlConnection: vi.fn(),
}));

vi.mock("@/lib/api/sso", () => ssoApi);

import SecuritySettingsPage from "./page";

describe("settings security five-card layout", () => {
  beforeEach(() => {
    ssoApi.getSamlConfig.mockResolvedValue({ configured: false, config: null });
  });

  it("renders all five security cards", async () => {
    render(<SecuritySettingsPage />);

    await waitFor(() => {
      expect(screen.getByTestId("saml-card")).toBeInTheDocument();
    });
    expect(screen.getByTestId("twofa-card")).toBeInTheDocument();
    expect(screen.getByTestId("ip-allowlist-card")).toBeInTheDocument();
    expect(screen.getByTestId("sessions-card")).toBeInTheDocument();
    expect(screen.getByTestId("audit-retention-card")).toBeInTheDocument();
  });

  it("gates the 2FA configure CTA with an honest v1.1 tooltip", async () => {
    render(<SecuritySettingsPage />);

    await waitFor(() => {
      expect(screen.getByTestId("twofa-card")).toBeInTheDocument();
    });
    const cta = screen.getByTestId("twofa-card-cta");
    expect(cta).toBeDisabled();
    expect(cta).toHaveAttribute(
      "title",
      "Per-user 2FA enrollment ships in v1.1. Contact sales for early access.",
    );
    expect(screen.getByTestId("twofa-card-tooltip")).toHaveTextContent(
      /ships in v1\.1/i,
    );
  });

  it("gates the IP allowlist CTA with an honest v1.1 tooltip", async () => {
    render(<SecuritySettingsPage />);

    await waitFor(() => {
      expect(screen.getByTestId("ip-allowlist-card")).toBeInTheDocument();
    });
    const cta = screen.getByTestId("ip-allowlist-card-cta");
    expect(cta).toBeDisabled();
    expect(cta).toHaveAttribute(
      "title",
      "IP allowlisting ships in v1.1. Contact sales for early access.",
    );
    expect(screen.getByTestId("ip-allowlist-card-rows")).toHaveTextContent(
      "0 ranges configured",
    );
  });

  it("renders the current session row + ships-in-v1.1 empty-state for sessions", async () => {
    render(<SecuritySettingsPage />);

    await waitFor(() => {
      expect(screen.getByTestId("sessions-card")).toBeInTheDocument();
    });
    expect(screen.getByTestId("sessions-current-row")).toHaveTextContent(
      "This browser (current session)",
    );
    expect(screen.getByTestId("sessions-empty")).toHaveTextContent(
      /Multi-device session revocation ships in v1\.1/i,
    );
  });

  it("gates the audit retention CTA with an enterprise tooltip", async () => {
    render(<SecuritySettingsPage />);

    await waitFor(() => {
      expect(screen.getByTestId("audit-retention-card")).toBeInTheDocument();
    });
    const cta = screen.getByTestId("audit-retention-card-cta");
    expect(cta).toBeDisabled();
    expect(cta).toHaveAttribute(
      "title",
      "Configurable retention policies are an enterprise feature. Contact sales.",
    );
    expect(screen.getByTestId("audit-retention-card-rows")).toHaveTextContent(
      /All audit rows retained indefinitely/i,
    );
  });

  it("preserves the existing SAML SSO card behaviour", async () => {
    render(<SecuritySettingsPage />);

    await waitFor(() => {
      expect(screen.getByTestId("saml-status-pill")).toHaveTextContent("Not configured");
    });
    expect(screen.getByTestId("saml-configure")).toHaveTextContent("Configure SAML");
    expect(screen.getByTestId("saml-metadata")).toHaveAttribute(
      "download",
      "skycot-sp-metadata.xml",
    );
  });
});
