import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  classifyRole,
  dispatchGenesisEvent,
  hireFromGenesis,
  parseSseBlock,
  streamGenesis,
  type GenesisErrorEvent,
  type GenesisProgressEvent,
  type GenesisStreamCallbacks,
  type GenesisVerticalReadyEvent,
} from "./genesis";
import { ApiError } from "./types";
import { STUB_QUESTIONS } from "@/lib/hire/genesis-stub";

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

const VALID_GATEWAY_RESPONSE = {
  classification: "marketplace-seller",
  questions: [
    {
      id: "q1",
      text: "How many messages do you get per day?",
      answer_type: "single-select",
      options: ["0–10", "10–50", "50+"],
    },
    {
      id: "q2",
      text: "What's the typical response you'd send?",
      answer_type: "free-text",
    },
  ],
};

describe("lib/api/genesis", () => {
  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
  });
  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    vi.restoreAllMocks();
  });

  it("returns gateway questions on 2xx happy path", async () => {
    installMock((req) => {
      if (req.url.endsWith("/api/v1/genesis/classify")) {
        return makeResponse({ status: 200, body: VALID_GATEWAY_RESPONSE });
      }
      return makeResponse({ status: 500 });
    });

    const result = await classifyRole({ problem: "I sell stuff on FB marketplace" });
    expect(result.source).toBe("gateway");
    expect(result.classification).toBe("marketplace-seller");
    expect(result.questions).toHaveLength(2);
    expect(result.questions[0]?.id).toBe("q1");
  });

  it("falls back to STUB_QUESTIONS on 404 (gateway not wired yet)", async () => {
    installMock(() => makeResponse({ status: 404, body: "" }));

    const result = await classifyRole({ problem: "anything" });
    expect(result.source).toBe("stub");
    expect(result.questions).toEqual([...STUB_QUESTIONS]);
  });

  it("re-throws ApiError on 5xx so caller can retry", async () => {
    installMock(() =>
      makeResponse({ status: 503, body: "boom", contentType: "text/plain" }),
    );
    await expect(classifyRole({ problem: "x" })).rejects.toBeInstanceOf(ApiError);
  });

  it("falls back to STUB_QUESTIONS on malformed response", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: { questions: [{ id: "bad", text: "missing answer_type" }] },
      }),
    );

    const result = await classifyRole({ problem: "x" });
    expect(result.source).toBe("stub");
    expect(result.questions).toEqual([...STUB_QUESTIONS]);
  });

  it("re-throws on network error (TypeError from fetch) so caller can retry", async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = typeof input === "string" ? input : (input as Request | URL).toString();
      if (url.endsWith("/api/auth/session")) {
        return Promise.resolve(makeResponse({ status: 200, body: {} }));
      }
      return Promise.reject(new TypeError("network unreachable"));
    });
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    await expect(classifyRole({ problem: "x" })).rejects.toThrow(TypeError);
  });

  it("rejects single-select questions with missing options as malformed", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          questions: [
            { id: "q1", text: "Pick one", answer_type: "single-select" },
          ],
        },
      }),
    );

    const result = await classifyRole({ problem: "x" });
    expect(result.source).toBe("stub");
  });

  it("rejects responses with too many questions (>3) as malformed", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          questions: [
            { id: "q1", text: "a", answer_type: "yes-no" },
            { id: "q2", text: "b", answer_type: "yes-no" },
            { id: "q3", text: "c", answer_type: "yes-no" },
            { id: "q4", text: "d", answer_type: "yes-no" },
          ],
        },
      }),
    );
    const result = await classifyRole({ problem: "x" });
    expect(result.source).toBe("stub");
  });
});

const READY_PAYLOAD: GenesisVerticalReadyEvent = {
  vertical_id: "customer-support-agent-mariah",
  vertical_dir_path: "/tmp/v/.genesis/op-x/customer-support-agent-mariah",
  tenant_id: "op-x",
  manifest: {
    id: "customer-support-agent-mariah",
    name: "Mariah",
    archetype: "customer_support_agent",
    persona: "mariah",
    actions_declared: 3,
  },
  prompts: ["prompts/system.md", "prompts/persona.md"],
  skills: [],
  eval_seed_count: 5,
  judge_verdict: "pass",
};

describe("parseSseBlock", () => {
  it("decodes event + JSON data", () => {
    const block = `event: progress\ndata: ${JSON.stringify({
      phase: "drafting_persona",
      message: "go",
      percent: 10,
    })}`;
    const parsed = parseSseBlock(block);
    expect(parsed?.event).toBe("progress");
    expect(parsed?.data).toEqual({
      phase: "drafting_persona",
      message: "go",
      percent: 10,
    });
  });

  it("ignores blocks without event or data", () => {
    expect(parseSseBlock("data: {}\n")).toBeNull();
    expect(parseSseBlock("event: progress\n")).toBeNull();
  });

  it("strips trailing carriage returns from line endings", () => {
    const block = "event: progress\r\ndata: {\"phase\":\"ready\",\"message\":\"go\",\"percent\":100}\r";
    const parsed = parseSseBlock(block);
    expect(parsed?.event).toBe("progress");
  });
});

describe("dispatchGenesisEvent", () => {
  function makeCallbacks(): {
    cb: GenesisStreamCallbacks;
    progress: GenesisProgressEvent[];
    ready: GenesisVerticalReadyEvent[];
    errors: GenesisErrorEvent[];
  } {
    const progress: GenesisProgressEvent[] = [];
    const ready: GenesisVerticalReadyEvent[] = [];
    const errors: GenesisErrorEvent[] = [];
    return {
      cb: {
        onProgress: (e) => progress.push(e),
        onVerticalReady: (e) => ready.push(e),
        onError: (e) => errors.push(e),
      },
      progress,
      ready,
      errors,
    };
  }

  it("routes progress, vertical_ready, and error events to the right callbacks", () => {
    const { cb, progress, ready, errors } = makeCallbacks();
    dispatchGenesisEvent(
      `event: progress\ndata: ${JSON.stringify({
        phase: "compiling_charter",
        message: "x",
        percent: 35,
      })}`,
      cb,
    );
    dispatchGenesisEvent(
      `event: vertical_ready\ndata: ${JSON.stringify(READY_PAYLOAD)}`,
      cb,
    );
    dispatchGenesisEvent(
      `event: error\ndata: ${JSON.stringify({
        phase: "validating_eval_seeds",
        message: "boom",
        recoverable: false,
      })}`,
      cb,
    );
    expect(progress).toHaveLength(1);
    expect(progress[0].phase).toBe("compiling_charter");
    expect(ready).toHaveLength(1);
    expect(ready[0].vertical_id).toBe("customer-support-agent-mariah");
    expect(errors).toHaveLength(1);
    expect(errors[0].message).toBe("boom");
  });

  it("ignores unknown event names", () => {
    const { cb, progress, ready, errors } = makeCallbacks();
    dispatchGenesisEvent(`event: chatter\ndata: {}`, cb);
    expect(progress).toHaveLength(0);
    expect(ready).toHaveLength(0);
    expect(errors).toHaveLength(0);
  });
});

describe("streamGenesis", () => {
  beforeEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
  });
  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
  });

  it("consumes a multi-event SSE stream and dispatches in order", async () => {
    const chunks = [
      `event: progress\ndata: ${JSON.stringify({
        phase: "drafting_persona",
        message: "go",
        percent: 10,
      })}\n\n`,
      `event: progress\ndata: ${JSON.stringify({
        phase: "compiling_charter",
        message: "ok",
        percent: 35,
      })}\n\n`,
      `event: vertical_ready\ndata: ${JSON.stringify(READY_PAYLOAD)}\n\n`,
    ];
    const encoder = new TextEncoder();
    const stream = new ReadableStream({
      start(controller) {
        for (const c of chunks) controller.enqueue(encoder.encode(c));
        controller.close();
      },
    });
    globalThis.fetch = vi.fn(() =>
      Promise.resolve(
        new Response(stream, {
          status: 200,
          headers: { "content-type": "text/event-stream" },
        }),
      ),
    ) as unknown as typeof fetch;

    const progress: GenesisProgressEvent[] = [];
    let ready: GenesisVerticalReadyEvent | null = null;
    await streamGenesis(
      {
        roleDescription: "Customer success lead who triages tier-1 tickets.",
        tenantId: "op-test",
      },
      {
        onProgress: (e) => progress.push(e),
        onVerticalReady: (e) => {
          ready = e;
        },
        onError: () => {
          throw new Error("unexpected error event");
        },
      },
    );
    expect(progress.map((p) => p.phase)).toEqual([
      "drafting_persona",
      "compiling_charter",
    ]);
    expect(ready).toEqual(READY_PAYLOAD);
  });

  it("throws ApiError on non-2xx", async () => {
    globalThis.fetch = vi.fn(() =>
      Promise.resolve(
        new Response("missing key", {
          status: 503,
          headers: { "content-type": "text/plain" },
        }),
      ),
    ) as unknown as typeof fetch;
    await expect(
      streamGenesis(
        { roleDescription: "x", tenantId: "op" },
        {
          onProgress: () => {},
          onVerticalReady: () => {},
          onError: () => {},
        },
      ),
    ).rejects.toBeInstanceOf(ApiError);
  });
});

describe("hireFromGenesis", () => {
  beforeEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
  });
  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
  });

  it("posts a canonical hire body and resolves to the worker id", async () => {
    let captured: { url: string; body: unknown; tenant: string | null } | null =
      null;
    installMock(async (req) => {
      captured = {
        url: req.url,
        body: req.body ? JSON.parse(await req.text()) : null,
        tenant: req.headers.get("X-Argo-Tenant-Id"),
      };
      return makeResponse({
        status: 201,
        body: { workerId: "wkr_42", success: true },
      });
    });
    const result = await hireFromGenesis({
      verticalReady: READY_PAYLOAD,
      roleDescription:
        "Customer success lead who triages tier-1 tickets daily.",
      tenantId: "op-test",
    });
    expect(result.workerId).toBe("wkr_42");
    expect(captured).not.toBeNull();
    const cap = captured as unknown as {
      url: string;
      body: Record<string, unknown>;
      tenant: string | null;
    };
    expect(cap.url).toContain("/api/v1/workers");
    expect(cap.tenant).toBe("op-test");
    const draft = cap.body.worker_draft as Record<string, unknown>;
    expect(draft.suggestedName).toBe("Mariah");
    expect(draft.suggestedVertical).toBe("customer-support-agent-mariah");
    expect((draft.charter as string).length).toBeGreaterThanOrEqual(20);
    expect(draft.approvalPolicy).toEqual({ level: "balanced" });
    const duties = draft.duties as Array<{ defaultEnabled: boolean }>;
    expect(duties.some((d) => d.defaultEnabled === true)).toBe(true);
  });

  it("accepts snake_case worker_id from the gateway response", async () => {
    installMock(() =>
      makeResponse({
        status: 201,
        body: { worker_id: "wkr_snake", success: true },
      }),
    );
    const result = await hireFromGenesis({
      verticalReady: READY_PAYLOAD,
      roleDescription:
        "Customer success lead who triages tier-1 tickets daily.",
      tenantId: "op-test",
    });
    expect(result.workerId).toBe("wkr_snake");
  });

  it("throws ApiError when the response shape is unexpected", async () => {
    installMock(() => makeResponse({ status: 201, body: { ok: true } }));
    await expect(
      hireFromGenesis({
        verticalReady: READY_PAYLOAD,
        roleDescription: "x".repeat(40),
        tenantId: "op",
      }),
    ).rejects.toBeInstanceOf(ApiError);
  });
});
