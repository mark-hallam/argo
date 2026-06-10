import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  AuditDetailNotFoundError,
  fetchAuditDetail,
} from "./audit-detail";
import { ApiError } from "./types";

const ORIGINAL_FETCH = globalThis.fetch;

// A canonical seed action id that `findSampleAuditDetail` resolves to a
// hand-shaped fixture (Priya's candidate-screen). Used for the mock-path
// assertions so we don't depend on the synthesised-from-row branch.
const SEED_ACTION_ID = "act_01J0PRIYA_001";

function makeResponse({
  status = 200,
  body,
  contentType = "application/json",
}: {
  status?: number;
  body?: unknown;
  contentType?: string | null;
}): Response {
  const headers = new Headers();
  if (contentType) headers.set("content-type", contentType);
  return new Response(
    body === undefined ? null : typeof body === "string" ? body : JSON.stringify(body),
    { status, statusText: `Status ${status}`, headers },
  );
}

function installMock(responder: (req: Request) => Response | Promise<Response>) {
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    if (url.endsWith("/api/auth/session")) {
      return Promise.resolve(makeResponse({ status: 200, body: {} }));
    }
    const req = new Request(url, init);
    return Promise.resolve(responder(req));
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return fetchMock;
}

function clearCookies() {
  for (const pair of document.cookie.split("; ")) {
    const name = pair.split("=")[0];
    if (name) document.cookie = `${name}=; max-age=0; path=/`;
  }
}

// A gateway drill-in envelope in the rich `summary`-shape so the normaliser
// exercises the full field-mapping path (including the hash-chain anchor).
function gatewayDetail(anchor: string) {
  return {
    summary: {
      id: SEED_ACTION_ID,
      timestamp: "2026-05-24T07:30:00.000Z",
      worker: { id: "wkr-live-1", name: "Live Compliance Worker" },
      action_type: "hipaa_compliance_check",
      verb: "ran HIPAA compliance check",
      status: "succeeded",
      channel: "web",
      data_classification: "restricted",
    },
    inputs: { trigger_kind: "message", trigger_summary: "Inbound webhook." },
    decisionChain: [{ n: 1, title: "Memory loaded", body: "context cached" }],
    provenance: {
      model: { id: "claude-opus-4-7", version: "2026-04-30" },
      skill: { id: "compliance-check", version: "1.0.0" },
      hash_chain_anchor: anchor,
    },
    output: { kind: "text", body: "ok" },
  };
}

describe("lib/api/audit-detail", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE;
    delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE;
    clearCookies();
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
    clearCookies();
  });

  describe("demo-live gate (per-user)", () => {
    it("an ANONYMOUS dev visitor under auth-bypass is served the fixture, no network", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      const fetchMock = installMock(() => makeResponse({ status: 500, body: "nope" }));

      const detail = await fetchAuditDetail(SEED_ACTION_ID);

      expect(detail.id).toBe(SEED_ACTION_ID);
      expect(detail.worker.name).toBe("Priya");
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("a REAL signed-in user is NEVER served the fixture — they hit the gateway", async () => {
      // Per-user gate: bypass flag set, but a real session cookie present.
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      document.cookie = "skycot-session=1; path=/";
      const fetchMock = installMock(() =>
        makeResponse({ status: 200, body: gatewayDetail("0xreal-user-anchor") }),
      );

      const detail = await fetchAuditDetail(SEED_ACTION_ID);

      // Gateway data, not the hand-shaped Priya fixture.
      expect(detail.worker.name).toBe("Live Compliance Worker");
      expect(fetchMock).toHaveBeenCalled();
    });

    it("demo-live opts an anonymous recording session OUT of the mock short-circuit", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
      const fetchMock = installMock(() =>
        makeResponse({ status: 200, body: gatewayDetail("0xdemo-live-anchor") }),
      );

      const detail = await fetchAuditDetail(SEED_ACTION_ID);

      expect(detail.worker.name).toBe("Live Compliance Worker");
      expect(fetchMock).toHaveBeenCalled();
    });
  });

  describe("hash-chain field shape", () => {
    it("surfaces the gateway's hash_chain_anchor through provenance on the happy path", async () => {
      installMock(() =>
        makeResponse({ status: 200, body: gatewayDetail("0xdeadbeefcafef00d") }),
      );

      const detail = await fetchAuditDetail(SEED_ACTION_ID);

      expect(detail.provenance.hash_chain_anchor).toBe("0xdeadbeefcafef00d");
      expect(detail.provenance.model.id).toBe("claude-opus-4-7");
      expect(detail.provenance.skill.version).toBe("1.0.0");
    });

    it("accepts the camelCase hashChainAnchor alias", async () => {
      const body = gatewayDetail("ignored") as Record<string, unknown>;
      (body.provenance as Record<string, unknown>) = {
        model: { id: "m", version: "1" },
        hashChainAnchor: "0xcamelcase",
      };
      installMock(() => makeResponse({ status: 200, body }));

      const detail = await fetchAuditDetail(SEED_ACTION_ID);

      expect(detail.provenance.hash_chain_anchor).toBe("0xcamelcase");
    });
  });

  describe("error / not-found handling", () => {
    it("maps a gateway 404 onto a typed AuditDetailNotFoundError", async () => {
      installMock(() =>
        makeResponse({
          status: 404,
          body: { error: { code: "not_found", message: "no such action" } },
        }),
      );

      await expect(fetchAuditDetail("act_ghost")).rejects.toBeInstanceOf(
        AuditDetailNotFoundError,
      );
    });

    it("rejects an empty action id without touching the network (missing row)", async () => {
      const fetchMock = installMock(() => makeResponse({ status: 200, body: {} }));

      await expect(fetchAuditDetail("")).rejects.toBeInstanceOf(
        AuditDetailNotFoundError,
      );
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("surfaces a tampered/unexpected gateway shape as a 502 ApiError, not a fabricated detail", async () => {
      // A non-record payload (array) is a contract violation — the normaliser
      // must refuse it rather than synthesise a fake provenance card.
      installMock(() => makeResponse({ status: 200, body: [] }));

      let caught: unknown;
      try {
        await fetchAuditDetail(SEED_ACTION_ID);
      } catch (err) {
        caught = err;
      }
      expect(caught).toBeInstanceOf(ApiError);
      expect((caught as ApiError).status).toBe(502);
    });

    it("propagates a 5xx outage so SREs see real gateway breakage", async () => {
      installMock(() =>
        makeResponse({ status: 503, body: "down", contentType: "text/plain" }),
      );

      let caught: unknown;
      try {
        await fetchAuditDetail(SEED_ACTION_ID);
      } catch (err) {
        caught = err;
      }
      expect(caught).toBeInstanceOf(ApiError);
      expect((caught as ApiError).status).toBe(503);
    });
  });
});
