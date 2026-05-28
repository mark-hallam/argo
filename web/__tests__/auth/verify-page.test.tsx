import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const replaceMock = vi.hoisted(() => vi.fn());
const searchMock = vi.hoisted(() => ({ current: new URLSearchParams("token=valid-token-abc123") }));

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: replaceMock,
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  useSearchParams: () => searchMock.current,
}));

import VerifyPage from "@/app/auth/verify/page";

const fetchMock = vi.fn();

beforeEach(() => {
  vi.stubGlobal("fetch", fetchMock);
  fetchMock.mockReset();
  replaceMock.mockReset();
  searchMock.current = new URLSearchParams("token=valid-token-abc123");
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

describe("VerifyPage", () => {
  it("redirects new users to /onboarding/welcome after verify", async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse(200, { ok: true, needs_onboarding: true, tenant_id: null }),
    );
    render(<VerifyPage />);
    await waitFor(() => {
      expect(replaceMock).toHaveBeenCalledWith("/onboarding/welcome");
    });
  });

  it("redirects existing tenants to the app root", async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse(200, { ok: true, needs_onboarding: false, tenant_id: "tnt_1" }),
    );
    render(<VerifyPage />);
    await waitFor(() => {
      expect(replaceMock).toHaveBeenCalledWith("/");
    });
  });

  it("shows a friendly error when the token has expired", async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse(410, {
        error: { code: "token_expired", message: "Sign-in link expired. Request a new one." },
      }),
    );
    render(<VerifyPage />);
    const err = await screen.findByTestId("verify-error");
    expect(err.textContent).toMatch(/expired/i);
    expect(replaceMock).not.toHaveBeenCalled();
  });

  it("renders the soft no-token landing when ?token= is absent", async () => {
    searchMock.current = new URLSearchParams("");
    render(<VerifyPage />);
    const body = await screen.findByTestId("verify-no-token-body");
    expect(body.textContent).toMatch(/magic link/i);
    expect(screen.queryByTestId("verify-error")).toBeNull();
    const primary = screen.getByTestId("verify-no-token-primary");
    expect(primary.getAttribute("href")).toBe("/auth/signin?from=verify");
    const secondary = screen.getByTestId("verify-no-token-secondary");
    expect(secondary.getAttribute("href")).toBe("/auth/signin");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("renders the soft no-token landing when ?token= is empty", async () => {
    searchMock.current = new URLSearchParams("token=");
    render(<VerifyPage />);
    const body = await screen.findByTestId("verify-no-token-body");
    expect(body.textContent).toMatch(/magic link/i);
    expect(screen.queryByTestId("verify-error")).toBeNull();
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("shows the 'didn't work' error when the token is present but invalid", async () => {
    searchMock.current = new URLSearchParams("token=not-a-real-token");
    fetchMock.mockResolvedValueOnce(
      jsonResponse(401, {
        error: { code: "invalid_token", message: "Sign-in link is no longer valid." },
      }),
    );
    render(<VerifyPage />);
    const err = await screen.findByTestId("verify-error");
    expect(err.textContent).toMatch(/no longer valid/i);
    expect(screen.queryByTestId("verify-no-token-body")).toBeNull();
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});
