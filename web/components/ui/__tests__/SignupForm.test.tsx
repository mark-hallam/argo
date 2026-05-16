import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SignupForm } from "../SignupForm";

describe("SignupForm", () => {
  it("submits the entered email and shows success state", async () => {
    const onSubmit = vi.fn();
    const user = userEvent.setup();
    const { rerender } = render(<SignupForm onSubmit={onSubmit} />);
    const input = screen.getByLabelText("Email address");
    await user.type(input, "user@example.com");
    await user.click(screen.getByRole("button", { name: /magic link/i }));
    expect(onSubmit).toHaveBeenCalledWith("user@example.com");

    rerender(<SignupForm onSubmit={onSubmit} state="success" />);
    expect(screen.getByRole("status")).toHaveTextContent(/inbox/i);
  });

  it("shows error message when state=error", () => {
    render(<SignupForm onSubmit={() => {}} state="error" errorMessage="Invalid email" />);
    expect(screen.getByRole("alert")).toHaveTextContent("Invalid email");
  });
});
