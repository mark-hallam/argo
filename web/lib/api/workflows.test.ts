import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchAllWorkflows,
  fetchWorkflow,
  parseStepsFromToml,
  type GatewayWorkflowItem,
} from "./workflows";
import { ApiError } from "./types";
import { SAMPLE_WORKFLOW_SPECS, SAMPLE_WORKFLOW_SUMMARIES } from "@/lib/sample-workflows";

const ORIGINAL_FETCH = globalThis.fetch;

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

const SAMPLE_TOML = `schema_version = "1"

[workflow]
name        = "post_listing"
description = "Validate inventory then post to Marketplace."

[[step]]
name = "trigger_in"
kind = "trigger"
description = "Seller posts inventory in Slack."

[[step]]
name = "validate"
kind = "skill_invoke"
description = "Run inventory.validate skill."

[[step]]
name = "seller_approves"
kind = "approval"
description = "Send draft to seller; one-tap approve."
`;

function makeItem(
  overrides: Partial<GatewayWorkflowItem> = {},
): GatewayWorkflowItem {
  return {
    id: "00000000-0000-0000-0000-000000000001",
    vertical_id: "gtm-engineer",
    name: "post_listing",
    version: 3,
    created_at: "2026-05-20T12:00:00Z",
    modified_by: "user:mariah",
    definition_toml: SAMPLE_TOML,
    ...overrides,
  };
}

describe("lib/api/workflows", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.ARGO_GATEWAY_AUTH_BYPASS;
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  describe("fetchAllWorkflows", () => {
    it("fans out across verticals and aggregates items on happy path", async () => {
      installMock((req) => {
        if (req.url.endsWith("/verticals")) {
          return makeResponse({
            status: 200,
            body: {
              ok: true,
              loaded: [{ id: "gtm-engineer" }, { id: "support" }],
              errors: [],
            },
          });
        }
        if (req.url.endsWith("/api/v1/verticals/gtm-engineer/workflows")) {
          return makeResponse({
            status: 200,
            body: { vertical_id: "gtm-engineer", count: 1, items: [makeItem()] },
          });
        }
        if (req.url.endsWith("/api/v1/verticals/support/workflows")) {
          return makeResponse({
            status: 200,
            body: {
              vertical_id: "support",
              count: 1,
              items: [makeItem({ vertical_id: "support", name: "triage_ticket" })],
            },
          });
        }
        return makeResponse({ status: 404, body: { error: { code: "x" } } });
      });
      const result = await fetchAllWorkflows();
      expect(result.source).toBe("gateway");
      expect(result.workflows.map((w) => `${w.vertical}/${w.name}`)).toEqual([
        "gtm-engineer/post_listing",
        "support/triage_ticket",
      ]);
      expect(result.workflows[0].steps).toBe(3);
      expect(result.workflows[0].risk).toBe("approval-required");
      expect(result.workflows[0].versions).toBe(3);
    });

    it("falls back to seed when /verticals returns empty loaded[]", async () => {
      installMock((req) => {
        if (req.url.endsWith("/verticals")) {
          return makeResponse({ status: 200, body: { ok: true, loaded: [] } });
        }
        return makeResponse({ status: 200, body: {} });
      });
      const result = await fetchAllWorkflows();
      expect(result.source).toBe("mock-offline");
      expect(result.workflows).toBe(SAMPLE_WORKFLOW_SUMMARIES);
    });

    it("falls back to seed when /verticals 404s in non-strict env", async () => {
      installMock(() =>
        makeResponse({
          status: 404,
          body: { error: { code: "not_found", message: "no verticals route" } },
        }),
      );
      const result = await fetchAllWorkflows();
      expect(result.source).toBe("mock-offline");
      expect(result.workflows).toBe(SAMPLE_WORKFLOW_SUMMARIES);
    });

    it("propagates /verticals 5xx in strict production", async () => {
      vi.stubEnv("NODE_ENV", "production");
      process.env.NEXT_PUBLIC_ENV = "production";
      installMock(() =>
        makeResponse({ status: 503, body: "gateway down", contentType: "text/plain" }),
      );
      let caught: unknown;
      try {
        await fetchAllWorkflows();
      } catch (err) {
        caught = err;
      }
      expect(caught).toBeInstanceOf(ApiError);
      expect((caught as ApiError).status).toBe(503);
    });

    it("returns mock-bypass without firing fetch when bypass flag set", async () => {
      process.env.ARGO_GATEWAY_AUTH_BYPASS = "1";
      const fetchMock = installMock(() => makeResponse({ status: 200, body: {} }));
      const result = await fetchAllWorkflows();
      expect(result.source).toBe("mock-bypass");
      expect(result.workflows).toBe(SAMPLE_WORKFLOW_SUMMARIES);
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("swallows per-vertical 404 / 5xx and keeps the rest", async () => {
      installMock((req) => {
        if (req.url.endsWith("/verticals")) {
          return makeResponse({
            status: 200,
            body: { loaded: [{ id: "alpha" }, { id: "beta" }] },
          });
        }
        if (req.url.endsWith("/api/v1/verticals/alpha/workflows")) {
          return makeResponse({ status: 404, body: { error: { code: "not_found" } } });
        }
        if (req.url.endsWith("/api/v1/verticals/beta/workflows")) {
          return makeResponse({
            status: 200,
            body: { vertical_id: "beta", count: 1, items: [makeItem({ vertical_id: "beta" })] },
          });
        }
        return makeResponse({ status: 500, body: "x", contentType: "text/plain" });
      });
      const result = await fetchAllWorkflows();
      expect(result.source).toBe("gateway");
      expect(result.workflows).toHaveLength(1);
      expect(result.workflows[0].vertical).toBe("beta");
    });
  });

  describe("fetchWorkflow", () => {
    it("decodes definition_toml on happy path", async () => {
      installMock(() => makeResponse({ status: 200, body: makeItem() }));
      const result = await fetchWorkflow("gtm-engineer", "post_listing");
      expect(result.source).toBe("gateway");
      expect(result.workflow?.steps).toHaveLength(3);
      expect(result.workflow?.steps[0].kind).toBe("trigger");
      expect(result.workflow?.version).toBe("v3");
      expect(result.workflow?.risk).toBe("approval-required");
    });

    it("falls back to seed when gateway 404s in non-strict env", async () => {
      installMock(() =>
        makeResponse({ status: 404, body: { error: { code: "not_found" } } }),
      );
      const result = await fetchWorkflow("support", "triage_ticket");
      expect(result.source).toBe("mock-offline");
      expect(result.workflow).toBe(SAMPLE_WORKFLOW_SPECS["support/triage_ticket"]);
    });

    it("returns mock-bypass without firing fetch when bypass flag set", async () => {
      process.env.ARGO_GATEWAY_AUTH_BYPASS = "1";
      const fetchMock = installMock(() => makeResponse({ status: 200, body: {} }));
      const result = await fetchWorkflow("support", "triage_ticket");
      expect(result.source).toBe("mock-bypass");
      expect(result.workflow).toBe(SAMPLE_WORKFLOW_SPECS["support/triage_ticket"]);
      expect(fetchMock).not.toHaveBeenCalled();
    });
  });

  describe("parseStepsFromToml", () => {
    it("extracts workflow description + step name/kind/description", () => {
      const parsed = parseStepsFromToml(SAMPLE_TOML);
      expect(parsed.description).toBe("Validate inventory then post to Marketplace.");
      expect(parsed.steps).toHaveLength(3);
      expect(parsed.steps[0]).toMatchObject({
        id: "step-1",
        kind: "trigger",
        name: "trigger_in",
      });
      expect(parsed.risk).toBe("approval-required");
    });

    it("falls back to tool_call for an unknown step kind", () => {
      const toml = `[workflow]\nname = "x"\n\n[[step]]\nname = "weird"\nkind = "spaceship"\n`;
      const parsed = parseStepsFromToml(toml);
      expect(parsed.steps[0].kind).toBe("tool_call");
    });

    it("returns empty step list + autonomous risk on a header-only doc", () => {
      const parsed = parseStepsFromToml("schema_version = \"1\"\n");
      expect(parsed.steps).toEqual([]);
      expect(parsed.risk).toBe("autonomous");
    });

    it("ignores `args = {…}` inline tables and unrelated sections", () => {
      const toml = `[workflow]\nname = "x"\ndescription = "desc"\n\n[[step]]\nname = "a"\nkind = "tool_call"\nargs = { path = "README.md" }\n\n[other]\nfoo = "bar"\n`;
      const parsed = parseStepsFromToml(toml);
      expect(parsed.description).toBe("desc");
      expect(parsed.steps).toHaveLength(1);
      expect(parsed.steps[0].kind).toBe("tool_call");
    });

    it("tolerates an empty / undefined toml safely", () => {
      const parsed = parseStepsFromToml("");
      expect(parsed.steps).toEqual([]);
      expect(parsed.description).toBe("");
    });
  });
});
