import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { draftWorker } from "@/lib/api/genesis";
import {
  GATEWAY_WORKER_DRAFT_FIELDS,
  fromGatewayWorkerDraft,
  type GatewayWorkerDraft,
} from "@/lib/hire/draft-types";
import { stubWorkerDraft } from "@/lib/hire/genesis-stub";

/**
 * Schema-mirror test for the genesis draft-worker response.
 *
 * The gateway-side Rust `WorkerDraft` struct
 * (`crates/argo-cli/src/handlers/gateway/routes_genesis.rs`, currently
 * private to the genesis handler) carries
 * `#[serde(rename_all = "camelCase")]` and returns a flat string-only
 * shape. The SPA used to expect a richer nested shape, so every
 * gateway hit silently failed the type-guard and fell back to
 * `stubWorkerDraft` — producing "5 hires → 5 Mariah clones." This
 * fixture pins the canonical field list so a future struct change
 * on the Rust side surfaces here.
 *
 * If you change the gateway struct, update both this fixture AND
 * [[GATEWAY_WORKER_DRAFT_FIELDS]] in `draft-types.ts`.
 */
const RUST_WORKER_DRAFT_FIELDS = [
  "suggestedName",
  "persona",
  "charter",
  "duties",
  "exampleReplies",
  "approvalPolicy",
  "suggestedVertical",
] as const;

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

function installMock(
  gatewayResponder: (req: Request) => Response | Promise<Response>,
) {
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    if (url.endsWith("/api/auth/session")) {
      return Promise.resolve(makeResponse({ status: 200, body: {} }));
    }
    const req = new Request(url, init);
    return Promise.resolve(gatewayResponder(req));
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return fetchMock;
}

function gatewayDraft(overrides: Partial<GatewayWorkerDraft> = {}): GatewayWorkerDraft {
  return {
    suggestedName: "Maya",
    persona: "warm marketplace concierge",
    charter:
      "Maya watches the inbox and replies to inbound questions with a friendly, on-brand tone.",
    duties: [
      "Reply to inbound marketplace questions",
      "Summarise overnight messages each morning",
    ],
    exampleReplies: [
      "Sure — I can confirm that's still available.",
      "Let me check on shipping and get back to you.",
    ],
    approvalPolicy:
      "1 of 3 actions require operator approval. Sensitive verbs need explicit operator OK before the worker can act.",
    suggestedVertical: "marketplace-seller",
    ...overrides,
  };
}

describe("GatewayWorkerDraft schema mirror", () => {
  it("declares the same camelCase field set as the Rust struct", () => {
    expect([...GATEWAY_WORKER_DRAFT_FIELDS].sort()).toEqual(
      [...RUST_WORKER_DRAFT_FIELDS].sort(),
    );
  });

  it("every canonical Rust field exists on the GatewayWorkerDraft TS interface", () => {
    const sample = gatewayDraft();
    for (const field of RUST_WORKER_DRAFT_FIELDS) {
      // Runtime: the sample carries every canonical field.
      expect(sample).toHaveProperty(field);
      // Compile-time: `keyof GatewayWorkerDraft` proves `field` is a key.
      // If a Rust field is added without updating the TS interface this
      // assertion narrows to `never` and the line below fails to compile.
      const _typed: keyof GatewayWorkerDraft = field;
      expect(typeof _typed).toBe("string");
    }
  });

  it("fromGatewayWorkerDraft populates every rich-shape field from the flat wire shape", () => {
    const rich = fromGatewayWorkerDraft(gatewayDraft());
    expect(rich.suggestedName).toBe("Maya");
    expect(rich.persona.tagline).toBe("warm marketplace concierge");
    expect(rich.persona.avatarKind).toBe("initials");
    expect(rich.charter).toMatch(/Maya watches the inbox/);
    expect(rich.duties).toHaveLength(2);
    expect(rich.duties[0]?.label).toBe("Reply to inbound marketplace questions");
    expect(rich.duties[0]?.defaultEnabled).toBe(true);
    expect(rich.exampleReplies).toHaveLength(2);
    expect(rich.exampleReplies[0]?.mariahResponse).toMatch(/still available/);
    expect(rich.approvalPolicy.level).toBe("balanced");
    expect(rich.suggestedVertical).toBe("marketplace-seller");
  });
});

describe("draftWorker live-gateway path", () => {
  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
  });
  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    vi.restoreAllMocks();
  });

  it("maps the gateway envelope to a rich WorkerDraft (source=gateway)", async () => {
    installMock((req) => {
      if (req.url.endsWith("/api/v1/genesis/draft-worker")) {
        return makeResponse({
          status: 200,
          body: { worker_draft: gatewayDraft(), cost_usd: null },
        });
      }
      return makeResponse({ status: 500 });
    });

    const result = await draftWorker({
      problem: "I sell on FB marketplace",
      answers: {},
    });
    expect(result.source).toBe("gateway");
    expect(result.draft.suggestedName).toBe("Maya");
    expect(result.draft.suggestedVertical).toBe("marketplace-seller");
    expect(result.draft.duties).toHaveLength(2);
  });

  it("produces different drafts for different inputs (no clone-the-stub regression)", async () => {
    const responder = (req: Request) => {
      if (!req.url.endsWith("/api/v1/genesis/draft-worker")) {
        return makeResponse({ status: 500 });
      }
      const callIndex = (responder as { calls?: number }).calls ?? 0;
      (responder as { calls?: number }).calls = callIndex + 1;
      const variant =
        callIndex === 0
          ? gatewayDraft({ suggestedName: "Ava", suggestedVertical: "marketplace-seller" })
          : gatewayDraft({ suggestedName: "Ren", suggestedVertical: "sdr-outbound" });
      return makeResponse({
        status: 200,
        body: { worker_draft: variant, cost_usd: 0.0021 },
      });
    };
    installMock(responder);

    const first = await draftWorker({ problem: "marketplace seller", answers: {} });
    const second = await draftWorker({ problem: "outbound SDR", answers: {} });
    expect(first.draft.suggestedName).not.toBe(second.draft.suggestedName);
    expect(first.draft.suggestedVertical).not.toBe(second.draft.suggestedVertical);
    expect(first.source).toBe("gateway");
    expect(second.source).toBe("gateway");
  });

  it("falls back to stub when the envelope is malformed (defensive)", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: { worker_draft: { suggestedName: "missing the rest" }, cost_usd: null },
      }),
    );
    const result = await draftWorker({ problem: "x", answers: {} });
    expect(result.source).toBe("stub");
    expect(result.draft).toEqual(stubWorkerDraft("x"));
  });

  it("still falls back to stub on gateway 404 (no Phase-2 banner regression)", async () => {
    installMock(() => makeResponse({ status: 404, body: "" }));
    const result = await draftWorker({ problem: "anything", answers: {} });
    expect(result.source).toBe("stub");
    expect(result.draft).toEqual(stubWorkerDraft("anything"));
  });
});
