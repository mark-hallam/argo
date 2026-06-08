import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CreateModal } from "./create-modal";

/**
 * FIX A (focus trap) + FIX D (custom-expiry validation + name maxLength)
 * for the Create API key dialog.
 */

function setup() {
  const onCancel = vi.fn();
  const onSubmit = vi.fn();
  render(
    <CreateModal
      onCancel={onCancel}
      onSubmit={onSubmit}
      isSubmitting={false}
      submitError={null}
    />,
  );
  return { onCancel, onSubmit };
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("CreateModal accessibility (FIX A)", () => {
  it("renders a labelled modal dialog and moves focus inside on open", () => {
    setup();
    const dialog = screen.getByRole("dialog", { name: "Create API key" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    // Focus trap lands on the first focusable control (the close button).
    expect(dialog.contains(document.activeElement)).toBe(true);
  });

  it("closes on Escape via the focus trap", async () => {
    const user = userEvent.setup();
    const { onCancel } = setup();
    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("caps the key name length", () => {
    setup();
    expect(screen.getByPlaceholderText(/CI pipeline/)).toHaveAttribute(
      "maxlength",
      "120",
    );
  });
});

describe("CreateModal custom-expiry validation (FIX D)", () => {
  it("a default (90d) preset submits without a custom date", async () => {
    const user = userEvent.setup();
    const { onSubmit } = setup();
    await user.type(screen.getByLabelText("Name"), "CI token");
    await user.click(screen.getByRole("button", { name: "Create key" }));
    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onSubmit.mock.calls[0]?.[0]?.expiry).toBe("90d");
  });

  it("blocks submit when Custom is selected with an empty date", async () => {
    const user = userEvent.setup();
    const { onSubmit } = setup();
    await user.type(screen.getByLabelText("Name"), "CI token");
    await user.click(screen.getByRole("radio", { name: "Custom" }));
    // Save must be disabled — empty custom date is not a valid future date.
    expect(screen.getByRole("button", { name: "Create key" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Create key" }));
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("blocks submit + shows an error for a past custom date", async () => {
    const user = userEvent.setup();
    const { onSubmit } = setup();
    await user.type(screen.getByLabelText("Name"), "CI token");
    await user.click(screen.getByRole("radio", { name: "Custom" }));
    const dateInput = screen.getByTestId("ak-custom-expiry");
    await user.type(dateInput, "2000-01-01");
    expect(screen.getByTestId("ak-custom-expiry-error")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Create key" })).toBeDisabled();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("allows submit with a future custom date and forwards it", async () => {
    const user = userEvent.setup();
    const { onSubmit } = setup();
    await user.type(screen.getByLabelText("Name"), "CI token");
    await user.click(screen.getByRole("radio", { name: "Custom" }));
    const future = "2999-12-31";
    await user.type(screen.getByTestId("ak-custom-expiry"), future);
    expect(screen.queryByTestId("ak-custom-expiry-error")).not.toBeInTheDocument();
    const submitBtn = screen.getByRole("button", { name: "Create key" });
    expect(submitBtn).not.toBeDisabled();
    await user.click(submitBtn);
    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onSubmit.mock.calls[0]?.[0]?.expiry).toEqual({ custom: future });
  });
});
