/**
 * Day-1 unblock FIX 1 — session tool-approval client.
 *
 * Pins the contract after migrating off the bespoke
 * `app/api/sessions/[id]/approve` Next proxy (empty NextAuth token in
 * prod → "Approve & run" 401) onto the shared authenticated `apiPost`
 * client: one `{ tool, note?, one_shot? }` POST per tool to
 * `/api/v1/sessions/<id>/approve` (rewritten to the gateway's bare
 * `/sessions/*`), grants returned in order, fail-fast on non-2xx.
 *
 * `apiPost` does a `/api/auth/session` peek first, so the mock returns an
 * empty session for that URL and routes the approve call to the
 * responder. Error shapes: the gateway's flat `{"error":"<string>"}`
 * surfaces the string as the ApiError `code` (message = statusText); a
 * nested `{"error":{code,message}}` surfaces `message`.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { approveSessionTool, approveSessionTools } from "./session-approvals";
import { ApiError } from "./types";

const ORIGINAL_FETCH = globalThis.fetch;
const SID = "019e6785-928a-7270-a766-f945b6cd7161";

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    statusText: `Status ${status}`,
    headers: { "content-type": "application/json" },
  });
}

function grantBody(tool: string) {
  return { ok: true, grant_id: `grant-${tool}`, session_id: SID, tool, one_shot: false };
}

/**
 * Install a fetch mock that routes the actual approve POST to
 * `responder` and answers `apiPost`'s internal plumbing fetches (the
 * `/api/auth/session` peek + any CSRF-token fetch) with benign
 * responses. The returned `approveCalls` records ONLY the approve
 * requests, so call-count assertions stay peek/CSRF-agnostic.
 */
function installMock(responder: (req: Request) => Response | Promise<Response>) {
  const approveCalls: Request[] = [];
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    if (url.includes("/sessions/") && url.includes("/approve")) {
      const req = new Request(url, init);
      approveCalls.push(req);
      return Promise.resolve(responder(req));
    }
    // apiPost session peek (`/api/auth/session`) + CSRF-token fetch — not
    // approve calls; answer benignly so the real request proceeds.
    return Promise.resolve(jsonResponse(200, {}));
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return approveCalls;
}

describe("lib/api/session-approvals", () => {
  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
  });
  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("POSTs { tool } to /api/v1/sessions/<id>/approve and returns the grant", async () => {
    const calls = installMock(async (req) => {
      expect(req.url).toMatch(new RegExp(`/api/v1/sessions/${SID}/approve$`));
      expect(req.method).toBe("POST");
      expect(await req.json()).toEqual({ tool: "delegate_task" });
      return jsonResponse(200, grantBody("delegate_task"));
    });

    const grant = await approveSessionTool(SID, "delegate_task");
    expect(grant.grant_id).toBe("grant-delegate_task");
    expect(grant.tool).toBe("delegate_task");
    expect(calls).toHaveLength(1);
  });

  it("forwards note and one_shot when supplied", async () => {
    installMock(async (req) => {
      expect(await req.json()).toEqual({
        tool: "send_email",
        note: "approved by mark",
        one_shot: true,
      });
      return jsonResponse(200, { ...grantBody("send_email"), one_shot: true });
    });

    const grant = await approveSessionTool(SID, "send_email", {
      note: "  approved by mark  ",
      oneShot: true,
    });
    expect(grant.one_shot).toBe(true);
  });

  it("approveSessionTools grants each tool sequentially, in order", async () => {
    const seen: string[] = [];
    installMock(async (req) => {
      const body = (await req.json()) as { tool: string };
      seen.push(body.tool);
      return jsonResponse(200, grantBody(body.tool));
    });

    const grants = await approveSessionTools(SID, ["web_search", "send_email"]);
    expect(seen).toEqual(["web_search", "send_email"]);
    expect(grants.map((g) => g.tool)).toEqual(["web_search", "send_email"]);
  });

  it("surfaces the gateway's flat string error envelope on the ApiError (code + status)", async () => {
    installMock(async () => jsonResponse(404, { error: `no session with id '${SID}'` }));

    const err = await approveSessionTool(SID, "send_email").catch((e) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(404);
    // apiPost's parseError surfaces the flat `{error:"<string>"}` as `code`
    // (message stays the HTTP statusText) and preserves the parsed body.
    expect((err as ApiError).code).toBe(`no session with id '${SID}'`);
  });

  it("surfaces a nested { error: { message } } gateway envelope as the ApiError message", async () => {
    installMock(async () =>
      jsonResponse(403, {
        error: { code: "forbidden", message: "role guest lacks approval.decide" },
      }),
    );

    const err = await approveSessionTool(SID, "send_email").catch((e) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).message).toBe("role guest lacks approval.decide");
  });

  it("approveSessionTools fails fast on the first error", async () => {
    let approveN = 0;
    const calls = installMock(async () => {
      approveN += 1;
      if (approveN === 1) return jsonResponse(200, grantBody("web_search"));
      return jsonResponse(500, { error: "add_session_grant: boom" });
    });

    await expect(
      approveSessionTools(SID, ["web_search", "send_email", "delegate_task"]),
    ).rejects.toBeInstanceOf(ApiError);
    expect(calls).toHaveLength(2);
  });
});
