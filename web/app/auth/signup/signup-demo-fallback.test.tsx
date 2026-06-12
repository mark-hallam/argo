/**
 * Acceptance test for `fix-demo-mode-cta-posture-and-error-copy` (shakedown
 * F9). When the gateway has demo mode switched off the `/api/auth/demo-mint`
 * proxy 404s (or returns a `demo_mode_off` envelope). The signup page must NOT
 * dead-end the prospect on a literal env-var instruction. Instead it:
 *
 *  1. Swaps the secondary "try a demo workspace" CTA for a human, brand-voice
 *     "the instant demo is napping" fallback panel.
 *  2. Lets the prospect join the early-access waitlist with the email they
 *     already typed (the email path stays usable).
 *  3. Reports a genuine waitlist failure inline rather than faking success.
 */

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), back: vi.fn() }),
  useSearchParams: () => new URLSearchParams(),
}));

// Keep `isValidEmail` real; control only the network call. `vi.hoisted`
// lets the mock factory (hoisted above imports) reference the spy safely.
const { joinWaitlistMock } = vi.hoisted(() => ({ joinWaitlistMock: vi.fn() }));
vi.mock("@/lib/api/waitlist", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/waitlist")>(
    "@/lib/api/waitlist",
  );
  return { ...actual, joinWaitlist: joinWaitlistMock };
});

import SignupPage from "./page";
import { ApiError } from "@/lib/api/types";

function demoOffResponse() {
  return {
    ok: false,
    status: 404,
    json: async () => ({
      error: {
        code: "demo_mode_off",
        message:
          "The instant demo is napping right now. Leave your email and we'll wake it for you.",
      },
    }),
  } as unknown as Response;
}

beforeEach(() => {
  // Force the secondary demo CTA visible regardless of NEXT_PUBLIC_ENV.
  vi.stubEnv("NEXT_PUBLIC_ALLOW_DEMO_MINT", "1");
  vi.stubEnv("NEXT_PUBLIC_ARGO_DEMO_MODE", "0");
  joinWaitlistMock.mockReset();
  global.fetch = vi.fn();
});

afterEach(() => {
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
});

async function clickDemoUntilNap() {
  const cta = await screen.findByText(
    /Or try a demo workspace · no email required/i,
  );
  fireEvent.click(cta);
  // The "napping" fallback replaces the CTA once the 404 lands.
  return waitFor(() => screen.getByTestId("signup-demo-nap"));
}

describe("signup demo-mode fallback (demo mode off)", () => {
  it("shows human fallback copy, never a raw env-var instruction", async () => {
    (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      demoOffResponse(),
    );
    render(<SignupPage />);

    const panel = await clickDemoUntilNap();
    expect(panel.textContent).toMatch(/napping/i);
    // The prospect must never see the literal env-var dead-end.
    expect(panel.textContent).not.toMatch(/ARGO_DEMO_MODE/);
    expect(screen.queryByText(/ARGO_DEMO_MODE/)).toBeNull();
  });

  it("joins the early-access waitlist with the typed email", async () => {
    (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      demoOffResponse(),
    );
    joinWaitlistMock.mockResolvedValue({ queued: true });
    render(<SignupPage />);

    const emailInput = screen.getByLabelText(/email/i) as HTMLInputElement;
    fireEvent.change(emailInput, { target: { value: "prospect@acme.com" } });

    await clickDemoUntilNap();
    fireEvent.click(screen.getByTestId("signup-demo-nap-join"));

    await waitFor(() =>
      expect(screen.getByText(/You're on the list/i)).toBeTruthy(),
    );
    expect(joinWaitlistMock).toHaveBeenCalledWith({
      email: "prospect@acme.com",
      source: "signup-demo-fallback",
    });
  });

  it("asks for an email before joining when none was typed", async () => {
    (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      demoOffResponse(),
    );
    render(<SignupPage />);

    await clickDemoUntilNap();
    fireEvent.click(screen.getByTestId("signup-demo-nap-join"));

    await waitFor(() =>
      expect(screen.getByTestId("signup-demo-nap-error").textContent).toMatch(
        /work email/i,
      ),
    );
    expect(joinWaitlistMock).not.toHaveBeenCalled();
  });

  it("surfaces a genuine waitlist failure instead of faking success", async () => {
    (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValue(
      demoOffResponse(),
    );
    joinWaitlistMock.mockRejectedValue(new ApiError(503, "Service down", "UPSTREAM"));
    render(<SignupPage />);

    const emailInput = screen.getByLabelText(/email/i) as HTMLInputElement;
    fireEvent.change(emailInput, { target: { value: "prospect@acme.com" } });

    await clickDemoUntilNap();
    fireEvent.click(screen.getByTestId("signup-demo-nap-join"));

    await waitFor(() =>
      expect(screen.getByTestId("signup-demo-nap-error").textContent).toMatch(
        /Service down/i,
      ),
    );
    expect(screen.queryByText(/You're on the list/i)).toBeNull();
  });
});
