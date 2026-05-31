import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const replaceMock = vi.hoisted(() => vi.fn());
const signInMock = vi.hoisted(() =>
  vi.fn(async () => ({ ok: true, error: null })),
);
const searchParamsMock = vi.hoisted(() => ({
  current: new URLSearchParams(),
}));

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: replaceMock,
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  useSearchParams: () => searchParamsMock.current,
}));

vi.mock("next-auth/react", () => ({
  signIn: signInMock,
}));

import { SIGNUP_REQUEST_TIMEOUT_MS } from "@/lib/auth/use-signup";
import SignupPage from "@/app/auth/signup/page";

const fetchMock = vi.fn();

beforeEach(() => {
  vi.stubGlobal("fetch", fetchMock);
  searchParamsMock.current = new URLSearchParams();
  fetchMock.mockReset();
  replaceMock.mockReset();
  signInMock.mockClear();
});

afterEach(() => {
  vi.useRealTimers();
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

  it("loads with a hire problem query without starting async signup work", () => {
    searchParamsMock.current = new URLSearchParams({
      returnTo: "/hire/step/2",
      problem: "Need help following up with leads",
    });

    render(<SignupPage />);

    expect(screen.getByTestId("signup-form")).toBeInTheDocument();
    expect(fetchMock).not.toHaveBeenCalled();
    expect(replaceMock).not.toHaveBeenCalled();
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

  it("settles the form when /api/auth/signup never responds", async () => {
    vi.useFakeTimers();
    fetchMock.mockReturnValueOnce(new Promise<Response>(() => {}));
    render(<SignupPage />);

    fireEvent.change(screen.getByTestId("signup-email"), {
      target: { value: "mark@example.com" },
    });
    fireEvent.click(screen.getByTestId("signup-submit"));

    expect(screen.getByTestId("signup-submit")).toBeDisabled();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SIGNUP_REQUEST_TIMEOUT_MS);
    });

    const alert = screen.getByRole("alert");
    expect(alert.textContent).toMatch(/took too long/i);
    expect(screen.getByTestId("signup-submit")).not.toBeDisabled();
    expect(replaceMock).not.toHaveBeenCalled();
  });
});

// Brief: fix-signup-demo-mode-env-flag-and-runbook-prerequisites.
// Asserts the "Or try a demo workspace" secondary button surfaces the
// demo path without an SMTP round-trip, and stays out of production
// builds unless explicitly opted-in via NEXT_PUBLIC_ALLOW_DEMO_MINT=1.
describe("SignupPage — demo-mint secondary button (runbook fallback)", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("renders the secondary demo-mint button by default in non-production builds", () => {
    // NEXT_PUBLIC_ENV is unset in test env (≠ "production") so the
    // button must show without any explicit override.
    render(<SignupPage />);
    expect(screen.getByTestId("signup-demo-mint")).toBeInTheDocument();
  });

  it("hides the secondary button on production builds unless NEXT_PUBLIC_ALLOW_DEMO_MINT=1", () => {
    vi.stubEnv("NEXT_PUBLIC_ENV", "production");
    const { unmount } = render(<SignupPage />);
    expect(screen.queryByTestId("signup-demo-mint")).not.toBeInTheDocument();
    unmount();

    vi.stubEnv("NEXT_PUBLIC_ALLOW_DEMO_MINT", "1");
    render(<SignupPage />);
    expect(screen.getByTestId("signup-demo-mint")).toBeInTheDocument();
  });

  it("clicking the secondary button calls /api/auth/demo-mint and never /api/auth/signup (SMTP-bypass)", async () => {
    const user = userEvent.setup();
    fetchMock.mockImplementation(async (url: RequestInfo | URL) => {
      const href = String(url);
      if (href.includes("/api/auth/demo-mint")) {
        return jsonResponse(200, {
          ok: true,
          principal: "human:demo@skycot.com",
          session_token: "demo.header.payload",
          needs_onboarding: true,
        });
      }
      throw new Error(`unexpected fetch call: ${href}`);
    });

    // Stub window.location.replace so JSDOM doesn't try to navigate
    // mid-assertion.
    const replaceLocation = vi.fn();
    Object.defineProperty(window, "location", {
      configurable: true,
      value: { ...window.location, replace: replaceLocation },
    });

    render(<SignupPage />);
    await user.type(screen.getByTestId("signup-email"), "demo@skycot.com");
    await user.click(screen.getByTestId("signup-demo-mint"));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalled();
    });
    const urlsCalled = fetchMock.mock.calls.map(([url]) => String(url));
    expect(urlsCalled).toContain("/api/auth/demo-mint");
    expect(urlsCalled).not.toContain("/api/auth/signup");
    await waitFor(() => {
      expect(replaceLocation).toHaveBeenCalledWith("/onboarding/welcome");
    });
  });

  it("settles the secondary demo-mint CTA when the mint request never responds", async () => {
    vi.useFakeTimers();
    fetchMock.mockReturnValueOnce(new Promise<Response>(() => {}));

    render(<SignupPage />);
    const button = screen.getByTestId("signup-demo-mint");
    fireEvent.click(button);

    expect(button).toBeDisabled();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SIGNUP_REQUEST_TIMEOUT_MS);
    });

    const alert = screen.getByRole("alert");
    expect(alert.textContent).toMatch(/demo workspace took too long/i);
    expect(button).not.toBeDisabled();
    expect(signInMock).not.toHaveBeenCalled();
  });
});
