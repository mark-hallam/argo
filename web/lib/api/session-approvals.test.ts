/**
 * Day-1 unblock FIX 1 — session tool-approval client.
 *
 * Pins the proxy contract: one `{ tool, note?, one_shot? }` POST per
 * tool to `/api/sessions/<id>/approve`, grant envelopes returned in
 * order, fail-fast with the gateway's error message on non-2xx. The
 * gateway answers errors as `{"error":"<string>"}` while the Next
 * proxy's own failure envelope is `{"error":{code,message}}` — both
 * must surface as the ApiError message.
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
  return {
    ok: true,
    grant_id: `grant-${tool}`,
    session_id: SID,
    tool,
    one_shot: false,
  };
}

describe("lib/api/session-approvals", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });
  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
  });

  it("POSTs { tool } to the session approve proxy and returns the grant", async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      expect(url).toBe(`/api/sessions/${SID}/approve`);
      expect(init?.method).toBe("POST");
      const body = JSON.parse(String(init?.body));
      expect(body).toEqual({ tool: "delegate_task" });
      return jsonResponse(200, grantBody("delegate_task"));
    });
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    const grant = await approveSessionTool(SID, "delegate_task");
    expect(grant.grant_id).toBe("grant-delegate_task");
    expect(grant.tool).toBe("delegate_task");
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("forwards note and one_shot when supplied", async () => {
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      const body = JSON.parse(String(init?.body));
      expect(body).toEqual({
        tool: "send_email",
        note: "approved by mark",
        one_shot: true,
      });
      return jsonResponse(200, { ...grantBody("send_email"), one_shot: true });
    });
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    const grant = await approveSessionTool(SID, "send_email", {
      note: "  approved by mark  ",
      oneShot: true,
    });
    expect(grant.one_shot).toBe(true);
  });

  it("approveSessionTools grants each tool sequentially, in order", async () => {
    const seen: string[] = [];
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      const body = JSON.parse(String(init?.body)) as { tool: string };
      seen.push(body.tool);
      return jsonResponse(200, grantBody(body.tool));
    });
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    const grants = await approveSessionTools(SID, ["web_search", "send_email"]);
    expect(seen).toEqual(["web_search", "send_email"]);
    expect(grants.map((g) => g.tool)).toEqual(["web_search", "send_email"]);
  });

  it("surfaces the gateway's string error envelope as the ApiError message", async () => {
    globalThis.fetch = vi.fn(async () =>
      jsonResponse(404, { error: `no session with id '${SID}'` }),
    ) as unknown as typeof fetch;

    const err = await approveSessionTool(SID, "send_email").catch((e) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(404);
    expect((err as ApiError).message).toBe(`no session with id '${SID}'`);
  });

  it("surfaces the proxy's { error: { message } } envelope as the ApiError message", async () => {
    globalThis.fetch = vi.fn(async () =>
      jsonResponse(502, {
        error: { code: "gateway_unreachable", message: "Couldn't reach the gateway." },
      }),
    ) as unknown as typeof fetch;

    const err = await approveSessionTool(SID, "send_email").catch((e) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).message).toBe("Couldn't reach the gateway.");
  });

  it("approveSessionTools fails fast on the first error", async () => {
    let calls = 0;
    globalThis.fetch = vi.fn(async () => {
      calls += 1;
      if (calls === 1) return jsonResponse(200, grantBody("web_search"));
      return jsonResponse(500, { error: "add_session_grant: boom" });
    }) as unknown as typeof fetch;

    await expect(
      approveSessionTools(SID, ["web_search", "send_email", "delegate_task"]),
    ).rejects.toBeInstanceOf(ApiError);
    expect(calls).toBe(2);
  });
});
