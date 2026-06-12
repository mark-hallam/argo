import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

// Mock the API client so the test exercises the form's render states, not fetch.
vi.mock("@/lib/api/waitlist", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/waitlist")>(
    "@/lib/api/waitlist",
  );
  return {
    ...actual,
    // Keep the real isValidEmail; stub only the network call.
    joinWaitlist: vi.fn(),
  };
});

import { joinWaitlist } from "@/lib/api/waitlist";
import { ApiError } from "@/lib/api/types";
import { WaitlistForm } from "@/components/marketing/WaitlistForm";

const joinWaitlistMock = vi.mocked(joinWaitlist);

afterEach(() => {
  vi.clearAllMocks();
});

describe("WaitlistForm", () => {
  it("renders the success card on a resolved joinWaitlist and never the 'not saved' copy", async () => {
    joinWaitlistMock.mockResolvedValueOnce({ queued: true });
    const user = userEvent.setup();
    render(<WaitlistForm source="landing-hero" />);

    await user.type(screen.getByTestId("waitlist-email"), "founder@acme.io");
    await user.click(screen.getByTestId("waitlist-submit"));

    const success = await screen.findByTestId("waitlist-success");
    expect(success).toBeInTheDocument();
    expect(screen.getByText(/you're on the list/i)).toBeInTheDocument();
    expect(joinWaitlistMock).toHaveBeenCalledWith({
      email: "founder@acme.io",
      source: "landing-hero",
    });

    // The stale draft-surface framing must never appear on a real success.
    expect(screen.queryByText(/your email was not saved/i)).toBeNull();
    expect(screen.queryByText(/isn't connected yet/i)).toBeNull();
  });

  it("retries once automatically, then shows a friendly error that retains the email on persistent failure", async () => {
    // Both attempts fail — the form must retry once, then surface the
    // friendly persistent-failure copy (not the raw 'Internal Server Error').
    joinWaitlistMock.mockRejectedValue(new ApiError(500, "Internal Server Error"));
    const user = userEvent.setup();
    render(<WaitlistForm source="landing-final" />);

    await user.type(screen.getByTestId("waitlist-email"), "founder@acme.io");
    await user.click(screen.getByTestId("waitlist-submit"));

    const err = await screen.findByTestId("waitlist-error", undefined, {
      timeout: 3000,
    });
    expect(err).toBeInTheDocument();
    // The single automatic retry means joinWaitlist was attempted twice.
    expect(joinWaitlistMock).toHaveBeenCalledTimes(2);
    // Friendly, lead-preserving copy — never the raw server string alone.
    expect(err).toHaveTextContent(/your email is saved here/i);
    // The lead is never lost: the entered email stays in the field.
    expect(screen.getByTestId("waitlist-email")).toHaveValue("founder@acme.io");
    // Never the stale draft copy, and never the success card on a fault.
    expect(screen.queryByText(/your email was not saved/i)).toBeNull();
    expect(screen.queryByText(/isn't connected yet/i)).toBeNull();
    expect(screen.queryByTestId("waitlist-success")).toBeNull();
  });

  it("recovers on the automatic retry when the first attempt was a transient blip", async () => {
    // First attempt rejects (gateway blip); the automatic retry succeeds —
    // the visitor sees the success card and never an error.
    joinWaitlistMock
      .mockRejectedValueOnce(new ApiError(503, "Service Unavailable"))
      .mockResolvedValueOnce({ queued: true });
    const user = userEvent.setup();
    render(<WaitlistForm source="landing-hero" />);

    await user.type(screen.getByTestId("waitlist-email"), "founder@acme.io");
    await user.click(screen.getByTestId("waitlist-submit"));

    const success = await screen.findByTestId("waitlist-success", undefined, {
      timeout: 3000,
    });
    expect(success).toBeInTheDocument();
    expect(joinWaitlistMock).toHaveBeenCalledTimes(2);
    // The transient error must never have been shown to the visitor.
    expect(screen.queryByTestId("waitlist-error")).toBeNull();
  });

  it("blocks an invalid email client-side without calling the route", async () => {
    const user = userEvent.setup();
    render(<WaitlistForm />);

    await user.type(screen.getByTestId("waitlist-email"), "nope");
    await user.click(screen.getByTestId("waitlist-submit"));

    await waitFor(() =>
      expect(screen.getByTestId("waitlist-error")).toHaveTextContent(
        /valid email/i,
      ),
    );
    expect(joinWaitlistMock).not.toHaveBeenCalled();
  });
});
