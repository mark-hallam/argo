import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { blobToBase64, pitchTurn, sttTranscribe } from "./voice";
import { ApiError } from "./types";

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

function installMock(responder: (req: Request) => Response) {
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

const OK_TURN = {
  transcript: "What does Skycot do?",
  answer_text: "Skycot is an AI-teammate platform.",
  audio_base64: "QUJD", // "ABC"
  format: "audio/mpeg",
  truth_boundary: {
    disposition: "Allow",
    source: "llm",
    block_reason: null,
    channel: "WebEmbed",
  },
};

describe("lib/api/voice — pitchTurn", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("posts audio_base64 to /voice/pitch-turn and returns the decoded turn", async () => {
    let captured: unknown = null;
    installMock((req) => {
      void req
        .clone()
        .text()
        .then((b) => {
          captured = b;
        });
      return makeResponse({ status: 200, body: OK_TURN });
    });
    const res = await pitchTurn({ audioBase64: "Zm9v", context: "WebEmbed" });
    await new Promise((r) => setTimeout(r, 0));
    expect(res.transcript).toBe("What does Skycot do?");
    expect(res.answer_text).toContain("AI-teammate");
    expect(res.audio_base64).toBe("QUJD");
    expect(res.truth_boundary?.disposition).toBe("Allow");
    expect(typeof captured).toBe("string");
    expect(captured as string).toContain('"audio_base64":"Zm9v"');
    expect(captured as string).toContain('"context":"WebEmbed"');
  });

  it("carries the truth boundary on a Block (over-claim defers) verbatim", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          transcript: "What's your valuation?",
          answer_text: "I won't share specific figures. Mark handles that directly.",
          audio_base64: "QUJD",
          format: "audio/mpeg",
          truth_boundary: {
            disposition: "Block",
            source: "canonical",
            block_reason: "forbidden_class: defer-to-mark-financial",
            channel: "WebEmbed",
          },
        },
      }),
    );
    const res = await pitchTurn({ audioBase64: "Zm9v" });
    expect(res.truth_boundary?.disposition).toBe("Block");
    expect(res.truth_boundary?.block_reason).toContain("defer-to-mark-financial");
    // The client never rewrites the answer; the deferral text passes through.
    expect(res.answer_text).not.toContain("$");
  });

  it("throws ApiError on a 503 (provider unavailable — never faked audio)", async () => {
    installMock(() =>
      makeResponse({
        status: 503,
        body: { error: { code: "voice_unavailable", message: "tts provider: no key" } },
      }),
    );
    await expect(pitchTurn({ audioBase64: "Zm9v" })).rejects.toBeInstanceOf(ApiError);
  });

  it("throws ApiError when the payload is missing required fields", async () => {
    installMock(() =>
      makeResponse({ status: 200, body: { transcript: "hi" } }), // no answer_text/audio
    );
    await expect(pitchTurn({ audioBase64: "Zm9v" })).rejects.toBeInstanceOf(ApiError);
  });

  it("throws ApiError when audio_base64 is empty (would be a silent no-audio)", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: { transcript: "hi", answer_text: "yo", audio_base64: "", format: "audio/mpeg" },
      }),
    );
    await expect(pitchTurn({ audioBase64: "Zm9v" })).rejects.toBeInstanceOf(ApiError);
  });

  it("omits voice_id/context from the body when not provided", async () => {
    let captured = "";
    installMock((req) => {
      void req
        .clone()
        .text()
        .then((b) => {
          captured = b;
        });
      return makeResponse({ status: 200, body: OK_TURN });
    });
    await pitchTurn({ audioBase64: "Zm9v" });
    await new Promise((r) => setTimeout(r, 0));
    expect(captured).toContain('"audio_base64":"Zm9v"');
    expect(captured).not.toContain("voice_id");
    expect(captured).not.toContain("context");
  });
});

describe("lib/api/voice — sttTranscribe", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("posts audio_base64 to /api/v1/voice/stt via apiPost and returns the transcript", async () => {
    let capturedUrl = "";
    let capturedBody = "";
    installMock((req) => {
      capturedUrl = req.url;
      void req
        .clone()
        .text()
        .then((b) => {
          capturedBody = b;
        });
      return makeResponse({
        status: 200,
        body: { transcript: "hire me a customer success lead", confidence: 0.94 },
      });
    });
    const res = await sttTranscribe("Zm9v");
    await new Promise((r) => setTimeout(r, 0));
    expect(res.transcript).toBe("hire me a customer success lead");
    expect(res.confidence).toBe(0.94);
    // The wrapper hits the deterministic /voice/stt route via apiPost (tenant
    // headers + cookie auth inherited from client.ts).
    expect(capturedUrl).toContain("/api/v1/voice/stt");
    expect(capturedBody).toContain('"audio_base64":"Zm9v"');
  });

  it("omits confidence when the provider doesn't report one", async () => {
    installMock(() => makeResponse({ status: 200, body: { transcript: "tier one tickets" } }));
    const res = await sttTranscribe("Zm9v");
    expect(res.transcript).toBe("tier one tickets");
    expect(res.confidence).toBeUndefined();
  });

  it("throws ApiError on a failed chunk (503 — never a faked transcript)", async () => {
    installMock(() =>
      makeResponse({
        status: 503,
        body: { error: { code: "voice_unavailable", message: "stt provider: no key" } },
      }),
    );
    await expect(sttTranscribe("Zm9v")).rejects.toBeInstanceOf(ApiError);
  });

  it("throws ApiError when the payload has no transcript field", async () => {
    installMock(() => makeResponse({ status: 200, body: { confidence: 0.5 } }));
    await expect(sttTranscribe("Zm9v")).rejects.toBeInstanceOf(ApiError);
  });
});

describe("lib/api/voice — blobToBase64", () => {
  it("strips the data-URL prefix and returns bare base64", async () => {
    const blob = new Blob([new Uint8Array([1, 2, 3, 4])], { type: "audio/webm" });
    const b64 = await blobToBase64(blob);
    // happy-dom's FileReader produces a real data URL; decoding must round-trip.
    expect(b64).not.toContain(",");
    expect(b64).not.toMatch(/^data:/);
    const decoded = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    expect(Array.from(decoded)).toEqual([1, 2, 3, 4]);
  });
});
