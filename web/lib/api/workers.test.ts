import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchWorker,
  hireWorker,
  listWorkersInVertical,
  triggerWorkerRun,
  WorkerNotFoundError,
} from "./workers";
import { fetchTeam } from "./team";
import {
  fetchWorkerOverview,
  WorkerOverviewNotFoundError,
} from "./worker-overview";
import { _clearDemoRosterForTests } from "./demo-roster-store";
import { SAMPLE_TEAM } from "@/lib/sample-team";
import { SAMPLE_WORKERS_DETAIL } from "@/lib/sample-workers-detail";
import type { WorkerDraft } from "@/lib/hire/draft-types";

const ORIGINAL_FETCH = globalThis.fetch;
const MARIAH = SAMPLE_WORKERS_DETAIL.find((worker) => worker.name === "Mariah")!;

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
    body === undefined
      ? null
      : typeof body === "string"
        ? body
        : JSON.stringify(body),
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

describe("lib/api/workers", () => {
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

  it("returns a bare worker response on the 2xx path", async () => {
    installMock(() => makeResponse({ body: MARIAH }));

    await expect(fetchWorker(MARIAH.id)).resolves.toEqual(MARIAH);
  });

  it("unwraps gateway { worker } envelopes", async () => {
    installMock(() => makeResponse({ body: { worker: MARIAH } }));

    await expect(fetchWorker(MARIAH.id)).resolves.toEqual(MARIAH);
  });

  it("falls back to bundled sample data for demo workers on dev 404", async () => {
    installMock((req) => {
      expect(req.url).toMatch(new RegExp(`/api/v1/workers/${MARIAH.id}$`));
      return makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "worker not found" } },
      });
    });

    await expect(fetchWorker(MARIAH.id)).resolves.toEqual(MARIAH);
  });

  it("throws WorkerNotFoundError for unknown ids after dev 404", async () => {
    installMock(() =>
      makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "worker not found" } },
      }),
    );

    await expect(
      fetchWorker("018f67d0-0000-7000-8000-000000009999"),
    ).rejects.toBeInstanceOf(WorkerNotFoundError);
  });

  // Brief: fix-fetchworker-400-bad-request-no-branch-infinite-retry.
  // The demo wizard mints non-UUID synthetic ids like
  // `demo-worker-marketing-intern-mpn1b10z`; the gateway rejects with
  // HTTP 400 ("invalid worker_id"). Before this branch landed, the raw
  // ApiError bubbled up + React Query retried indefinitely + the layout
  // stayed in `isPending` for ~15s.
  it("translates HTTP 400 (invalid_worker_id) to WorkerNotFoundError", async () => {
    installMock(() =>
      makeResponse({
        status: 400,
        body: {
          error: {
            code: "bad_request",
            message: "invalid worker_id 'demo-worker-marketing-intern-mpn1b10z'",
          },
        },
      }),
    );

    await expect(
      fetchWorker("demo-worker-marketing-intern-mpn1b10z"),
    ).rejects.toBeInstanceOf(WorkerNotFoundError);
  });

  it("falls back to bundled sample data for demo workers on dev 400", async () => {
    installMock((req) => {
      expect(req.url).toMatch(new RegExp(`/api/v1/workers/${MARIAH.id}$`));
      return makeResponse({
        status: 400,
        body: { error: { code: "bad_request", message: "invalid worker_id" } },
      });
    });

    await expect(fetchWorker(MARIAH.id)).resolves.toEqual(MARIAH);
  });

  // Day-1 unblock FIX 1: the run envelope's ADDITIVE probation fields
  // (`session_id`, `held_tools`, `probation`) must ride through
  // triggerWorkerRun so the overview can render the held state — and
  // an older gateway that omits them must still parse.
  it("triggerWorkerRun passes session_id / held_tools / probation through", async () => {
    installMock((req) => {
      expect(req.url).toMatch(/\/api\/workers\/w-1\/run$/);
      return makeResponse({
        body: {
          action_id: "a1",
          audit_id: "a1",
          trigger_id: "t1",
          queued_at: "2026-06-11T00:00:00Z",
          session_id: "019e6785-928a-7270-a766-f945b6cd7161",
          held_tools: ["send_email", "web_search"],
          probation: true,
        },
      });
    });

    const result = await triggerWorkerRun("w-1");
    expect(result.session_id).toBe("019e6785-928a-7270-a766-f945b6cd7161");
    expect(result.held_tools).toEqual(["send_email", "web_search"]);
    expect(result.probation).toBe(true);
  });

  it("triggerWorkerRun tolerates an older gateway envelope without the probation fields", async () => {
    installMock(() =>
      makeResponse({
        body: {
          action_id: "a2",
          audit_id: "a2",
          trigger_id: "t2",
          queued_at: "2026-06-11T00:00:00Z",
        },
      }),
    );

    const result = await triggerWorkerRun("w-1");
    expect(result.action_id).toBe("a2");
    expect(result.held_tools).toBeUndefined();
    expect(result.probation).toBeUndefined();
  });

  // Keystone "Approve & run" fix: the operator's held-tool list rides
  // on the run request as `approve_tools` so the gateway mints one-shot
  // grants on the NEW run's session — and the response's
  // `approved_tools` reports what was actually granted.
  it("triggerWorkerRun sends approve_tools and parses approved_tools", async () => {
    installMock(async (req) => {
      expect(req.url).toMatch(/\/api\/workers\/w-1\/run$/);
      const body = (await req.json()) as Record<string, unknown>;
      expect(body.approve_tools).toEqual(["send_email", "web_search"]);
      expect(body.trigger_source).toBe("manual_run_now");
      return makeResponse({
        body: {
          action_id: "a3",
          audit_id: "a3",
          trigger_id: "t3",
          queued_at: "2026-06-11T00:00:00Z",
          session_id: "019e6785-928a-7270-a766-f945b6cd7162",
          held_tools: [],
          probation: true,
          approved_tools: ["send_email", "web_search"],
        },
      });
    });

    const result = await triggerWorkerRun("w-1", undefined, [
      "send_email",
      "web_search",
    ]);
    expect(result.approved_tools).toEqual(["send_email", "web_search"]);
    expect(result.held_tools).toEqual([]);
  });

  it("triggerWorkerRun omits approve_tools when none are given", async () => {
    installMock(async (req) => {
      const body = (await req.json()) as Record<string, unknown>;
      expect(body).not.toHaveProperty("approve_tools");
      return makeResponse({
        body: {
          action_id: "a4",
          audit_id: "a4",
          trigger_id: "t4",
          queued_at: "2026-06-11T00:00:00Z",
        },
      });
    });

    const result = await triggerWorkerRun("w-1", undefined, []);
    expect(result.action_id).toBe("a4");
  });
});

const UUIDV7_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function buildHireDraft(overrides: Partial<WorkerDraft> = {}): WorkerDraft {
  return {
    suggestedName: "Mira",
    persona: {
      tagline: "answers buyer messages",
      avatarKind: "initials",
      voiceProfile: { tone: "warm", formality: "medium", signaturePhrases: [] },
    },
    charter:
      "Mira helps the Procurement team keep marketplace conversations moving.",
    duties: [
      {
        label: "Reply to buyer questions",
        schedule: { kind: "event", spec: "message.inbound" },
        defaultEnabled: true,
      },
    ],
    exampleReplies: [],
    approvalPolicy: { level: "balanced" },
    suggestedVertical: "marketplace",
    ...overrides,
  };
}

describe("lib/api/workers — demo-mode roster", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    _clearDemoRosterForTests();
    vi.stubEnv("NODE_ENV", "development");
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    delete process.env.NEXT_PUBLIC_ENV;
  });

  afterEach(() => {
    _clearDemoRosterForTests();
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("hireWorker returns a real-shaped UUIDv7 in auth-bypass mode (not stub-prefixed)", async () => {
    const result = await hireWorker({
      workerDraft: buildHireDraft(),
      problem: "buyer messages are piling up",
      answers: {},
    });
    expect(result.success).toBe(true);
    expect(result.workerId.startsWith("stub-")).toBe(false);
    expect(result.workerId).toMatch(UUIDV7_RE);
  });

  it("fetchTeam includes the new hire after hireWorker in auth-bypass mode", async () => {
    const before = await fetchTeam();
    expect(before.workers.length).toBe(SAMPLE_TEAM.length);
    expect(before.source).toBe("mock-bypass");

    const hire = await hireWorker({
      workerDraft: buildHireDraft({ suggestedName: "Mira" }),
      problem: "buyer messages are piling up",
      answers: {},
    });

    const after = await fetchTeam();
    expect(after.workers.length).toBe(SAMPLE_TEAM.length + 1);
    const newHire = after.workers.find((w) => w.id === hire.workerId);
    expect(newHire).toBeDefined();
    expect(newHire?.name).toBe("Mira");
  });

  it("fetchWorkerOverview resolves the just-hired worker to a non-404 card", async () => {
    const hire = await hireWorker({
      workerDraft: buildHireDraft({ suggestedName: "Mira" }),
      problem: "buyer messages are piling up",
      answers: {},
    });
    const result = await fetchWorkerOverview(hire.workerId);
    expect(result.source).toBe("mock-bypass");
    expect(result.overview.charter).toContain("Procurement");
    expect(result.overview.duties[0]?.label).toBe("Reply to buyer questions");
  });

  // Brief: fix-workers-unknown-id-renders-mock-instead-of-404 (F12).
  // In auth-bypass mode the gateway is mocked, so an unseeded UUID
  // must throw WorkerNotFoundError instead of fabricating a worker —
  // no theater for a curious investor pasting a URL.
  it("fetchWorker throws WorkerNotFoundError for unknown ids in auth-bypass", async () => {
    const fetchMock = vi.fn();
    globalThis.fetch = fetchMock as unknown as typeof fetch;
    await expect(
      fetchWorker("00000000-0000-0000-0000-000000000000"),
    ).rejects.toBeInstanceOf(WorkerNotFoundError);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("fetchWorkerOverview throws WorkerOverviewNotFoundError for unknown ids in auth-bypass", async () => {
    const fetchMock = vi.fn();
    globalThis.fetch = fetchMock as unknown as typeof fetch;
    await expect(
      fetchWorkerOverview("00000000-0000-0000-0000-000000000000"),
    ).rejects.toBeInstanceOf(WorkerOverviewNotFoundError);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("session-local roster wipes on clear (proxy for tab-close scoping)", async () => {
    await hireWorker({
      workerDraft: buildHireDraft(),
      problem: "queue piling up",
      answers: {},
    });
    const withHire = await fetchTeam();
    expect(withHire.workers.length).toBe(SAMPLE_TEAM.length + 1);

    _clearDemoRosterForTests();
    const afterClear = await fetchTeam();
    expect(afterClear.workers.length).toBe(SAMPLE_TEAM.length);
  });
});

// ws3-3 (sprint-ws3-3-verify-prod-no-bypass-fetchers): the
// workflow→schedule drawer's `listWorkersInVertical` had a 200-OK-but-
// unrecognized-shape branch that returned the bundled sample roster
// UNCONDITIONALLY — a real signed-in production user could see fixture
// workers. These pin the fix: prod gets an honest empty list, only the
// dev/bypass path renders the sample roster (NO THEATER).
describe("lib/api/workers — ws3-3: listWorkersInVertical never serves fixtures in prod", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("returns the live worker array from a { workers } envelope under NODE_ENV=production", async () => {
    vi.stubEnv("NODE_ENV", "production");
    const fetchMock = installMock(() =>
      makeResponse({ body: { workers: [MARIAH] } }),
    );
    const result = await listWorkersInVertical(MARIAH.role.vertical);
    expect(fetchMock).toHaveBeenCalled();
    expect(result).toEqual([MARIAH]);
  });

  it("returns an honest empty list (NOT sample fixtures) when a prod 200 has an unrecognized shape", async () => {
    vi.stubEnv("NODE_ENV", "production");
    // 200 OK, but neither a bare array nor a { workers } envelope.
    installMock(() => makeResponse({ body: { unexpected: true } }));
    const result = await listWorkersInVertical(MARIAH.role.vertical);
    expect(result).toEqual([]);
    const sampleIds = new Set(SAMPLE_WORKERS_DETAIL.map((w) => w.id));
    expect(result.some((w) => sampleIds.has(w.id))).toBe(false);
  });

  it("still renders the bundled sample roster on an unrecognized shape in dev (bypass behavior preserved)", async () => {
    vi.stubEnv("NODE_ENV", "development");
    installMock(() => makeResponse({ body: { unexpected: true } }));
    const result = await listWorkersInVertical(MARIAH.role.vertical);
    const expected = SAMPLE_WORKERS_DETAIL.filter(
      (w) => w.role.vertical === MARIAH.role.vertical,
    );
    expect(result).toEqual(expected);
    expect(result.length).toBeGreaterThan(0);
  });
});
