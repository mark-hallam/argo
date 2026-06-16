import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// `vi.hoisted` so the mock factory (hoisted above imports) can reference the
// shared cookie store without a TDZ error.
const { cookieStore } = vi.hoisted(() => ({
  cookieStore: { get: vi.fn() },
}));
vi.mock("next/headers", () => ({
  cookies: vi.fn(async () => cookieStore),
}));

import { POST } from "./route";

const SESSION_JWT = "header.payload.signature";

describe("POST /api/auth/logout (same-origin cookie clear)", () => {
  beforeEach(() => {
    cookieStore.get.mockReset();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("clears both __Host- cookies and returns 200 ok", async () => {
    cookieStore.get.mockReturnValue({ value: SESSION_JWT });
    const fetchMock = vi.fn(
      async (_input: RequestInfo | URL, _init?: RequestInit) =>
        new Response(null, { status: 200 }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const res = await POST();

    expect(res.status).toBe(200);
    await expect(res.json()).resolves.toEqual({ ok: true });

    const setCookies = res.headers.getSetCookie();
    expect(
      setCookies.some((c) => c.startsWith("__Host-session=;") && /Max-Age=0/.test(c)),
    ).toBe(true);
    expect(
      setCookies.some((c) => c.startsWith("__Host-csrf=;") && /Max-Age=0/.test(c)),
    ).toBe(true);

    // Best-effort revoke: the gateway logout is called with the session
    // cookie forwarded server-to-server (no browser CORS).
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const init = fetchMock.mock.calls[0]?.[1];
    expect(init).toBeDefined();
    expect((init!.headers as Record<string, string>).Cookie).toContain(
      `__Host-session=${SESSION_JWT}`,
    );
  });

  it("still clears cookies + returns 200 when the gateway revoke throws", async () => {
    cookieStore.get.mockReturnValue({ value: SESSION_JWT });
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new Error("gateway unreachable");
      }),
    );

    const res = await POST();

    expect(res.status).toBe(200);
    await expect(res.json()).resolves.toEqual({ ok: true });
    expect(res.headers.getSetCookie()).toHaveLength(2);
  });

  it("skips the gateway call when there is no session cookie but still clears", async () => {
    cookieStore.get.mockReturnValue(undefined);
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    const res = await POST();

    expect(res.status).toBe(200);
    expect(fetchMock).not.toHaveBeenCalled();
    expect(res.headers.getSetCookie()).toHaveLength(2);
  });
});
