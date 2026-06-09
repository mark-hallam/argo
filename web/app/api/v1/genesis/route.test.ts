import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { NextRequest } from "next/server";
import { POST } from "./route";

// R4-F1 regression pins. The Genesis generate is proxied by THIS route handler
// (it shadows Next's generic `/api/v1/:path*` rewrite for `/api/v1/genesis` only,
// to keep the SSE stream unbuffered). The bug: the handler hand-picked a header
// subset and DROPPED `Cookie`, so the generate reached the gateway WITHOUT the
// `__Host-session` cookie → the gateway resolved the cwd/anonymous tenant and
// wrote the vertical under `.genesis/<cwd>/`, while the follow-up hire (via the
// cookie-forwarding rewrite) resolved the SIGNED-IN tenant and searched
// `.genesis/<session>/` → `400 unknown vertical`. The fix forwards the session +
// CSRF identity so both endpoints resolve the SAME tenant by construction.
//
// `Cookie` is a forbidden request header, so a spec-compliant `Request`/
// `NextRequest` constructor (happy-dom here, undici in prod) silently drops it —
// we therefore feed headers through a faithful fake whose only surface is the
// handler's real usage (`req.headers.get(...)` + `req.text()`).

const SESSION_JWT = "header.payload.signature";
const CSRF_TOKEN = "csrf-double-submit-token-value";
const COOKIE = `__Host-session=${SESSION_JWT}; __Host-csrf=${CSRF_TOKEN}`;

function fakeRequest(headers: Record<string, string>): NextRequest {
  const lower: Record<string, string> = {};
  for (const [k, v] of Object.entries(headers)) lower[k.toLowerCase()] = v;
  return {
    headers: { get: (k: string) => lower[k.toLowerCase()] ?? null },
    text: async () =>
      JSON.stringify({ role_description: "A billing operations assistant." }),
  } as unknown as NextRequest;
}

/** A minimal, already-closed SSE stream so the handler's read loop completes. */
function upstreamOk(): Response {
  const stream = new ReadableStream<Uint8Array>({ start: (c) => c.close() });
  return new Response(stream, {
    status: 200,
    headers: { "content-type": "text/event-stream" },
  });
}

/** Pull the `Headers` the handler forwarded upstream to the gateway. */
function forwardedHeaders(spy: { mock: { calls: unknown[][] } }): Headers {
  const call = spy.mock.calls.find((args) =>
    String(args[0]).endsWith("/api/v1/genesis"),
  );
  if (!call) throw new Error("gateway fetch was never made");
  return new Headers((call[1] as RequestInit | undefined)?.headers);
}

describe("POST /api/v1/genesis proxy — identity forwarding (R4-F1)", () => {
  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://127.0.0.1:7400";
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("forwards the __Host-session cookie so the generate writes under the signed-in tenant", async () => {
    const fetchSpy = vi
      .spyOn(globalThis, "fetch")
      .mockImplementation(async () => upstreamOk());

    await POST(fakeRequest({ cookie: COOKIE, "x-csrf-token": CSRF_TOKEN }));

    const fwd = forwardedHeaders(fetchSpy);
    expect(fwd.get("cookie")).toBe(COOKIE);
    expect(fwd.get("cookie")).toContain("__Host-session=");
  });

  it("forwards the matching X-CSRF-Token so the cookie-auth POST passes the gateway CSRF gate", async () => {
    const fetchSpy = vi
      .spyOn(globalThis, "fetch")
      .mockImplementation(async () => upstreamOk());

    await POST(fakeRequest({ cookie: COOKIE, "x-csrf-token": CSRF_TOKEN }));

    expect(forwardedHeaders(fetchSpy).get("x-csrf-token")).toBe(CSRF_TOKEN);
  });

  it("still forwards X-Argo-Tenant-Id (unchanged behaviour) alongside the new identity headers", async () => {
    const fetchSpy = vi
      .spyOn(globalThis, "fetch")
      .mockImplementation(async () => upstreamOk());

    await POST(
      fakeRequest({
        cookie: COOKIE,
        "x-csrf-token": CSRF_TOKEN,
        "x-argo-tenant-id": "op-display-id",
      }),
    );

    expect(forwardedHeaders(fetchSpy).get("x-argo-tenant-id")).toBe(
      "op-display-id",
    );
  });

  it("anonymous demo path: no cookie present → nothing forwarded, gateway falls to cwd (both generate + hire match)", async () => {
    const fetchSpy = vi
      .spyOn(globalThis, "fetch")
      .mockImplementation(async () => upstreamOk());

    await POST(fakeRequest({ "x-argo-tenant-id": "anonymous" }));

    const fwd = forwardedHeaders(fetchSpy);
    expect(fwd.get("cookie")).toBeNull();
    expect(fwd.get("x-csrf-token")).toBeNull();
    // Tenant header still rides along for the audit/display trail.
    expect(fwd.get("x-argo-tenant-id")).toBe("anonymous");
  });
});
