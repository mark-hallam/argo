import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const replaceMock = vi.hoisted(() => vi.fn());

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: replaceMock,
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  useSearchParams: () => new URLSearchParams(),
}));

import SignupPage from "@/app/auth/signup/page";

const fetchMock = vi.fn();

beforeEach(() => {
  vi.stubGlobal("fetch", fetchMock);
  fetchMock.mockReset();
  replaceMock.mockReset();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("SignupPage", () => {
  it("renders the magic-link form with brand-consistent layout", () => {
    render(<SignupPage />);
    expect(screen.getByTestId("signup-form")).toBeInTheDocument();
    expect(screen.getByTestId("signup-email")).toHaveAttribute("type", "email");
    expect(screen.getByTestId("signup-submit")).toBeDisabled();
  });

  it("associates the 'Work email' label with the email input via htmlFor", () => {
    render(<SignupPage />);
    // getByLabelText only finds the input when label[htmlFor] matches input[id].
    // This is the screen-reader association the WCAG label rule requires.
    const labeled = screen.getByLabelText("Work email");
    expect(labeled).toBe(screen.getByTestId("signup-email"));
    expect(labeled).toHaveAttribute("id", "signup-email-input");
  });

  it("submitting a valid email POSTs to /api/auth/signup and redirects to check-email", async () => {
    const user = userEvent.setup();
    fetchMock.mockResolvedValueOnce(jsonResponse(200, { status: "sent" }));
    render(<SignupPage />);

    await user.type(screen.getByTestId("signup-email"), "Mark@Example.com");
    await user.click(screen.getByTestId("signup-submit"));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledTimes(1);
    });
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/auth/signup");
    expect(JSON.parse((init as RequestInit).body as string)).toEqual({
      email: "mark@example.com",
    });
    await waitFor(() => {
      expect(replaceMock).toHaveBeenCalledWith(
        "/auth/check-email?email=mark%40example.com",
      );
    });
  });

  it("shows an inline error when the email is invalid", async () => {
    const user = userEvent.setup();
    render(<SignupPage />);
    await user.type(screen.getByTestId("signup-email"), "not-an-email");
    await user.click(screen.getByTestId("signup-submit"));
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toMatch(/valid email/i);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("surfaces the rate-limit message when the backend returns 429", async () => {
    const user = userEvent.setup();
    fetchMock.mockResolvedValueOnce(
      jsonResponse(429, {
        error: { code: "rate_limited", message: "Too many attempts. Try again in an hour." },
      }),
    );
    render(<SignupPage />);
    await user.type(screen.getByTestId("signup-email"), "mark@example.com");
    await user.click(screen.getByTestId("signup-submit"));
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toMatch(/Too many attempts/);
    expect(replaceMock).not.toHaveBeenCalled();
  });
});
