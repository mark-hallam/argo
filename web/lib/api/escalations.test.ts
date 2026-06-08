import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchWorkerEscalations,
  updateWorkerEscalations,
} from "./escalations";

const ORIGINAL_FETCH = globalThis.fetch;

function makeResponse(body: unknown, status = 200): Response {
  const headers = new Headers();
  headers.set("content-type", "application/json");
  return new Response(JSON.stringify(body), {
    status,
    statusText: `Status ${status}`,
    headers,
  });
}

function installMock(responder: (url: string) => Response) {
  const fetchMock = vi.fn((input: RequestInfo | URL) => {
    const url =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    if (url.endsWith("/api/auth/session")) {
      return Promise.resolve(makeResponse({}));
    }
    return Promise.resolve(responder(url));
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return fetchMock;
}

const LIVE_PAYLOAD = {
  worker_id: "w1",
  read_only: false,
  is_default: false,
  rules: [
    {
      action: "Issue a refund",
      condition: "Refund exceeds $200",
      routes_to: "Finance",
      channel: "Email",
    },
    {
      action: "Close a ticket",
      condition: "Customer mentions churn",
      routes_to: "Account manager",
      channel: null,
    },
  ],
};

describe("lib/api/escalations", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE;
    delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE;
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("fetches the live endpoint and maps snake_case rules to the SPA contract", async () => {
    const fetchMock = installMock((url) => {
      expect(url).toContain("/api/v1/workers/w1/escalations");
      return makeResponse(LIVE_PAYLOAD);
    });

    const { policy, source } = await fetchWorkerEscalations("w1");
    expect(source).toBe("gateway");
    expect(fetchMock).toHaveBeenCalled();
    expect(policy.isDefault).toBe(false);
    expect(policy.readOnly).toBe(false);
    expect(policy.rules).toHaveLength(2);
    expect(policy.rules[0]).toEqual({
      action: "Issue a refund",
      condition: "Refund exceeds $200",
      routesTo: "Finance",
      channel: "Email",
    });
    // null channel normalizes to undefined.
    expect(policy.rules[1].channel).toBeUndefined();
  });

  it("calls the live endpoint under demo-live even with auth-bypass set", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    const fetchMock = installMock(() => makeResponse(LIVE_PAYLOAD));

    const { source } = await fetchWorkerEscalations("w1");
    expect(source).toBe("gateway");
    expect(fetchMock).toHaveBeenCalled();
  });

  it("short-circuits to the sample under anonymous auth-bypass (no gateway call)", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    const fetchMock = installMock(() => makeResponse(LIVE_PAYLOAD));

    const { policy, source } = await fetchWorkerEscalations("w1");
    expect(source).toBe("mock-bypass");
    expect(policy.rules.length).toBeGreaterThan(0);
    // No escalations call should have fired.
    const escalationCalls = fetchMock.mock.calls.filter(([input]) => {
      const url = typeof input === "string" ? input : (input as URL).toString();
      return url.includes("/escalations");
    });
    expect(escalationCalls).toHaveLength(0);
  });

  it("falls back to the sample on a 404 from the live endpoint", async () => {
    installMock(() => makeResponse({ error: "not found" }, 404));

    const { policy, source } = await fetchWorkerEscalations("missing");
    expect(source).toBe("mock-offline");
    expect(policy.rules.length).toBeGreaterThan(0);
  });

  it("PATCHes the rule set with a snake_case wire body", async () => {
    let captured: { url: string; body: string } | undefined;
    const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url =
        typeof input === "string"
          ? input
          : input instanceof URL
            ? input.toString()
            : input.url;
      if (url.endsWith("/api/auth/session")) {
        return Promise.resolve(makeResponse({}));
      }
      captured = { url, body: String(init?.body ?? "") };
      return Promise.resolve(makeResponse(LIVE_PAYLOAD));
    });
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    const policy = await updateWorkerEscalations("w1", {
      rules: [
        {
          action: "Issue a refund",
          condition: "Refund exceeds $200",
          routesTo: "Finance",
          channel: "Email",
        },
      ],
      operatorNote: "tightened refund threshold",
    });

    expect(captured?.url).toContain("/api/v1/workers/w1/escalations");
    const sent = JSON.parse(captured?.body ?? "{}");
    expect(sent.operator_note).toBe("tightened refund threshold");
    expect(sent.rules[0].routes_to).toBe("Finance");
    expect(sent.rules[0].channel).toBe("Email");
    // Response is mapped back to the SPA contract.
    expect(policy.rules[0].routesTo).toBe("Finance");
  });
});
