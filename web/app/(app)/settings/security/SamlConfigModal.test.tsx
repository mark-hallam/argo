import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { SamlConfig } from "@/lib/api/sso";

const saveSamlConfig = vi.fn();
const testSamlConnection = vi.fn();

vi.mock("@/lib/api/sso", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api/sso")>();
  return {
    ...actual,
    saveSamlConfig: (input: SamlConfig) => saveSamlConfig(input),
    testSamlConnection: (input: SamlConfig) => testSamlConnection(input),
  };
});

import SamlConfigModal from "./SamlConfigModal";

const VALID: SamlConfig = {
  idpEntityId: "http://www.okta.com/abc123",
  ssoUrl: "https://example.okta.com/app/x/sso/saml",
  certificate: "-----BEGIN CERTIFICATE-----\nMIID\n-----END CERTIFICATE-----",
  flavor: "okta",
  allowOwnerBypass: true,
};

const EMPTY: SamlConfig = {
  idpEntityId: "",
  ssoUrl: "",
  certificate: "",
  flavor: "okta",
  allowOwnerBypass: true,
};

function setup(initial: SamlConfig) {
  const onCancel = vi.fn();
  const onSaved = vi.fn();
  render(<SamlConfigModal initial={initial} onCancel={onCancel} onSaved={onSaved} />);
  return { onCancel, onSaved };
}

afterEach(() => {
  vi.clearAllMocks();
  document.body.innerHTML = "";
});

describe("SamlConfigModal accessibility (FIX A)", () => {
  it("moves focus into the dialog on open and closes on Escape", async () => {
    const user = userEvent.setup();
    const { onCancel } = setup(EMPTY);
    const dialog = screen.getByRole("dialog", { name: "Configure SAML SSO" });
    expect(dialog.contains(document.activeElement)).toBe(true);
    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});

describe("SamlConfigModal validation (FIX D)", () => {
  it("disables Save until entity, SSO, and certificate are filled", () => {
    setup(EMPTY);
    expect(screen.getByTestId("saml-save")).toBeDisabled();
  });

  it("flags invalid attribute-mapping JSON and disables Save", () => {
    setup(VALID);
    const attr = screen.getByPlaceholderText(/schemas.skycot\/role/);
    // fireEvent.change avoids userEvent's `{`/`[` keyboard-syntax parsing.
    fireEvent.change(attr, { target: { value: "not json" } });
    expect(screen.getByTestId("saml-attr-error")).toBeInTheDocument();
    expect(screen.getByTestId("saml-save")).toBeDisabled();
  });

  it("rejects a JSON array (must be an object)", () => {
    setup(VALID);
    const attr = screen.getByPlaceholderText(/schemas.skycot\/role/);
    fireEvent.change(attr, { target: { value: "[1, 2, 3]" } });
    expect(screen.getByTestId("saml-attr-error")).toHaveTextContent(
      /must be a JSON object/,
    );
    expect(screen.getByTestId("saml-save")).toBeDisabled();
  });

  it("accepts valid JSON and trims identifiers before saving", async () => {
    const user = userEvent.setup();
    saveSamlConfig.mockResolvedValue(VALID);
    // Start from valid required fields but with padded identifiers.
    setup({ ...VALID, idpEntityId: "  pad-entity  ", ssoUrl: "  https://sso  " });
    const attr = screen.getByPlaceholderText(/schemas.skycot\/role/);
    fireEvent.change(attr, { target: { value: '{"role":"r"}' } });
    expect(screen.queryByTestId("saml-attr-error")).not.toBeInTheDocument();

    const save = screen.getByTestId("saml-save");
    expect(save).not.toBeDisabled();
    await user.click(save);

    await waitFor(() => expect(saveSamlConfig).toHaveBeenCalledTimes(1));
    const sent = saveSamlConfig.mock.calls[0]?.[0] as SamlConfig;
    expect(sent.idpEntityId).toBe("pad-entity");
    expect(sent.ssoUrl).toBe("https://sso");
  });
});
