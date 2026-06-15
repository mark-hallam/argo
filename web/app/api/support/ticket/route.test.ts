import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NextRequest } from "next/server";

// A realistic gateway bearer token. If this string ever appears in the
// outbound support-webhook payload, the submitter's full gateway authority
// has leaked to a third party (MF-4).
const LEAKABLE_TOKEN =
  "skycot-session-jwt-eyJ0eXAiOiJKV1QiL0123456789abcdef-do-not-leak";
const SUBMITTER = "person@example.com";
const WEBHOOK_URL = "https://support.example.com/webhook";

// Mock the NextAuth session so the route handler sees an authenticated
// submitter whose session carries a gateway token.
vi.mock("@/auth", () => ({
  auth: vi.fn(async () => ({ gatewayToken: LEAKABLE_TOKEN })),
}));

// Pin the gateway URL so the persistTicket fetch is identifiable.
vi.mock("@/lib/config/gateway-url", () => ({
  resolveGatewayUrl: () => "http://127.0.0.1:7400",
}));

// Imported after the mocks are registered.
import { POST } from "./route";

interface CapturedRequest {
  url: string;
  init: RequestInit | undefined;
}

function makeRequest(body: unknown): NextRequest {
  return new NextRequest("http://localhost:3000/api/support/ticket", {
    method: "POST",
    headers: { "content-type": "application/json", "user-agent": "vitest" },
    body: JSON.stringify(body),
  });
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

let captured: CapturedRequest[];

beforeEach(() => {
  captured = [];
  process.env.SUPPORT_WEBHOOK_URL = WEBHOOK_URL;
  delete process.env.SUPPORT_WEBHOOK_TOKEN;
  vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
    const url = typeof input === "string" ? input : input.toString();
    captured.push({ url, init });
    if (url.endsWith("/api/v1/support/tickets")) {
      return jsonResponse({ ticket_id: "tkt-123" });
    }
    if (url === WEBHOOK_URL) {
      return jsonResponse({ ok: true });
    }
    throw new Error(`unexpected fetch: ${url}`);
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  delete process.env.SUPPORT_WEBHOOK_URL;
});

function outboundWebhookBody(): { email?: string; context?: Record<string, unknown> } {
  const call = captured.find((c) => c.url === WEBHOOK_URL);
  expect(call, "expected an outbound webhook fetch").toBeDefined();
  return JSON.parse(String(call?.init?.body)) as {
    email?: string;
    context?: Record<string, unknown>;
  };
}

describe("POST /api/support/ticket — MF-4 token-leak regression", () => {
  it("does NOT forward the submitter's gateway token in the webhook payload", async () => {
    const res = await POST(
      makeRequest({ email: SUBMITTER, message: "My agent is stuck." }),
    );
    expect(res.status).toBe(202);

    const body = outboundWebhookBody();
    // The payload identifies the user by email + ticket id, never by token.
    expect(body.email).toBe(SUBMITTER);
    expect(body.context).toBeDefined();
    expect(body.context).not.toHaveProperty("gateway_token");
  });

  it("never serializes the raw token anywhere in the outbound webhook body", async () => {
    await POST(makeRequest({ email: SUBMITTER, message: "Help please." }));
    const call = captured.find((c) => c.url === WEBHOOK_URL);
    expect(String(call?.init?.body)).not.toContain(LEAKABLE_TOKEN);
  });

  it("still includes the non-sensitive identifying context (ticket_id + submitted_at)", async () => {
    await POST(makeRequest({ email: SUBMITTER, message: "Status?" }));
    const body = outboundWebhookBody();
    expect(body.context?.ticket_id).toBe("tkt-123");
    expect(typeof body.context?.submitted_at).toBe("string");
  });
});
