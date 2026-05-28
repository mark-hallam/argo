import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Closes onboarding-hire-an-ai-teammate F6 [P1] —
 * `/landing` → "Hire your first teammate" CTA must not let an
 * unauthenticated visitor author a whole AI teammate before the
 * gateway 401s on the Step-4 commit POST.
 *
 * The fix routes the Step-1 Continue handler through
 * `/auth/signup?returnTo=/hire/step/2&problem=...` when no session
 * is present, and the signup page persists the problem to the
 * hire-draft localStorage so the wizard rehydrates after the
 * magic-link verify lands the user back at `/hire/step/2`.
 */

const pushMock = vi.hoisted(() => vi.fn());
const replaceMock = vi.hoisted(() => vi.fn());

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: pushMock,
    replace: replaceMock,
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  useSearchParams: () => new URLSearchParams(),
}));

const useSessionMock = vi.hoisted(() => vi.fn());

vi.mock("next-auth/react", () => ({
  useSession: () => useSessionMock(),
}));

import HireLandingPage from "@/app/(hire)/hire/page";
import { HIRE_DRAFT_STORAGE_KEY } from "@/lib/hire/draft-types";

function installMemoryStorage(): Storage {
  const data = new Map<string, string>();
  const storage: Storage = {
    get length() {
      return data.size;
    },
    clear: () => data.clear(),
    getItem: (k) => (data.has(k) ? (data.get(k) as string) : null),
    key: (i) => Array.from(data.keys())[i] ?? null,
    removeItem: (k) => {
      data.delete(k);
    },
    setItem: (k, v) => {
      data.set(k, String(v));
    },
  };
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: storage,
  });
  return storage;
}

const ROLE_DESCRIPTION =
  "I need someone to triage incoming customer email and reply to FAQs.";

beforeEach(() => {
  installMemoryStorage();
  pushMock.mockReset();
  replaceMock.mockReset();
  useSessionMock.mockReset();
  // The wizard's auth-bypass detector reads NEXT_PUBLIC_* env vars at
  // module-eval time; tests stub them per-case so the default test
  // environment behaves like a production-style deployment.
  vi.unstubAllEnvs();
});

afterEach(() => {
  window.localStorage.clear();
  vi.unstubAllEnvs();
});

describe("Hire wizard auth guard — Step 1", () => {
  it("routes an unauthenticated visitor to /auth/signup with returnTo + problem", async () => {
    useSessionMock.mockReturnValue({ data: null, status: "unauthenticated" });
    const user = userEvent.setup();
    render(<HireLandingPage />);

    // Step1Land hydrates the textarea once the draft store loads;
    // the form is keyed by the harness on `id="hire-problem"`.
    const textarea = await screen.findByRole("textbox");
    await user.click(textarea);
    await user.keyboard(ROLE_DESCRIPTION);

    // The footer Continue button lives in the editorial shell; the
    // role/name shortcut surfaces it without coupling to layout.
    const continueButton = await screen.findByRole("button", {
      name: /continue/i,
    });
    await user.click(continueButton);

    await waitFor(() => {
      expect(pushMock).toHaveBeenCalledTimes(1);
    });
    const target = pushMock.mock.calls[0][0] as string;
    expect(target.startsWith("/auth/signup?")).toBe(true);
    const qs = new URLSearchParams(target.split("?")[1] ?? "");
    expect(qs.get("returnTo")).toBe("/hire/step/2");
    expect(qs.get("problem")).toBe(ROLE_DESCRIPTION);
  });

  it("uses the existing /hire/step/2 push for an authenticated visitor", async () => {
    useSessionMock.mockReturnValue({
      data: { user: { email: "buyer@example.com" } },
      status: "authenticated",
    });
    const user = userEvent.setup();
    render(<HireLandingPage />);

    const textarea = await screen.findByRole("textbox");
    await user.click(textarea);
    await user.keyboard(ROLE_DESCRIPTION);
    await user.click(
      await screen.findByRole("button", { name: /continue/i }),
    );

    await waitFor(() => {
      expect(pushMock).toHaveBeenCalledWith("/hire/step/2");
    });
    const targets = pushMock.mock.calls.map(([t]) => String(t));
    expect(targets.every((t) => !t.startsWith("/auth/signup"))).toBe(true);
  });

  it("skips the signup redirect in auth-bypass / demo modes", async () => {
    vi.stubEnv("NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS", "1");
    useSessionMock.mockReturnValue({ data: null, status: "unauthenticated" });
    const user = userEvent.setup();
    render(<HireLandingPage />);

    const textarea = await screen.findByRole("textbox");
    await user.click(textarea);
    await user.keyboard(ROLE_DESCRIPTION);
    await user.click(
      await screen.findByRole("button", { name: /continue/i }),
    );

    await waitFor(() => {
      expect(pushMock).toHaveBeenCalledWith("/hire/step/2");
    });
    const targets = pushMock.mock.calls.map(([t]) => String(t));
    expect(targets.every((t) => !t.startsWith("/auth/signup"))).toBe(true);
  });

  it("persists the problem to hire-draft localStorage before redirecting", async () => {
    useSessionMock.mockReturnValue({ data: null, status: "unauthenticated" });
    const user = userEvent.setup();
    render(<HireLandingPage />);

    const textarea = await screen.findByRole("textbox");
    await user.click(textarea);
    await user.keyboard(ROLE_DESCRIPTION);
    await user.click(
      await screen.findByRole("button", { name: /continue/i }),
    );

    await waitFor(() => {
      expect(pushMock).toHaveBeenCalledTimes(1);
    });
    const raw = window.localStorage.getItem(HIRE_DRAFT_STORAGE_KEY);
    expect(raw).not.toBeNull();
    const parsed = JSON.parse(raw as string) as { problem?: string };
    expect(parsed.problem).toBe(ROLE_DESCRIPTION);
  });
});
