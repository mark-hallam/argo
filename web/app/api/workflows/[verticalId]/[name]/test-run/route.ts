import { type NextRequest } from "next/server";
import { auth } from "@/auth";

/**
 * Test-run streaming endpoint for the visual workflow builder.
 *
 * Streams newline-delimited JSON (NDJSON) events that drive the
 * canvas overlay: a "step:start" pulse, a "step:end" success/fail,
 * and a final "run:summary" with totals. Each event is one line:
 *
 *   {"type":"step:start","stepId":"s1","ts":...}
 *   {"type":"step:end","stepId":"s1","ok":true,"durationMs":418,"tokens":312,"response":"…"}
 *   {"type":"run:summary","ok":6,"fail":0,"durationMs":4218,"costUsd":0.014}
 *
 * V1 ships the route as a mock simulator. When the gateway lands
 * an SSE / NDJSON workflow-execution endpoint, swap the
 * `streamFromGateway` block — the on-wire event shape is stable.
 *
 * NDJSON (not SSE) keeps the parser trivial on both sides and works
 * with `fetch().body.getReader()` without an EventSource client.
 */

const DEFAULT_GATEWAY = "http://127.0.0.1:7400";

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

interface StepDescriptor {
  id: string;
  name: string;
  kind: string;
}

interface RunBody {
  steps?: StepDescriptor[];
  failStepId?: string | null;
}

async function streamFromGateway(
  verticalId: string,
  name: string,
  body: RunBody,
  gatewayToken: string,
): Promise<Response | null> {
  try {
    const upstream = await fetch(
      `${gatewayBase()}/api/v1/workflows/${encodeURIComponent(verticalId)}/${encodeURIComponent(name)}/runs?stream=true`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/x-ndjson",
          Authorization: `Bearer ${gatewayToken}`,
        },
        body: JSON.stringify(body),
        cache: "no-store",
      },
    );
    if (!upstream.ok || !upstream.body) return null;
    return new Response(upstream.body, {
      status: 200,
      headers: {
        "Content-Type": "application/x-ndjson; charset=utf-8",
        "X-Test-Run-Source": "gateway",
      },
    });
  } catch {
    return null;
  }
}

const SAMPLE_RESPONSES: Record<string, string> = {
  trigger: "Trigger fired — payload normalized.",
  skill_invoke:
    "Skill returned structured output. Confidence: 0.92. No PII detected.",
  approval: "Approval requested via channel. (Simulated auto-approve in test.)",
  tool_call: "Tool call completed. Status: 200 OK. Idempotency-key set.",
  decision: "Branch chosen based on score band.",
  notify: "Notification delivered.",
  data: "Persisted to ATS / store. Tag applied.",
};

function buildMockStream(body: RunBody): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  const steps = Array.isArray(body.steps) ? body.steps : [];
  const failId = body.failStepId ?? null;
  let cancelled = false;

  return new ReadableStream<Uint8Array>({
    async start(controller) {
      const send = (obj: unknown) => {
        if (cancelled) return;
        controller.enqueue(encoder.encode(JSON.stringify(obj) + "\n"));
      };
      const delay = (ms: number) =>
        new Promise<void>((resolve) => setTimeout(resolve, ms));

      let totalMs = 0;
      let okCount = 0;
      let failCount = 0;
      let totalTokens = 0;
      let halted = false;

      for (const step of steps) {
        if (cancelled || halted) break;
        const startTs = Date.now();
        send({ type: "step:start", stepId: step.id, ts: startTs });

        const stepLatencyMs = 380 + Math.floor(Math.random() * 520);
        await delay(stepLatencyMs);
        if (cancelled) break;
        totalMs += stepLatencyMs;

        if (failId && step.id === failId) {
          failCount += 1;
          send({
            type: "step:end",
            stepId: step.id,
            ok: false,
            durationMs: stepLatencyMs,
            error: `Simulated failure on ${step.name}. (Test-run injected fault.)`,
          });
          halted = true;
          break;
        }

        const tokens =
          step.kind === "skill_invoke"
            ? 180 + Math.floor(Math.random() * 280)
            : 0;
        totalTokens += tokens;
        okCount += 1;
        send({
          type: "step:end",
          stepId: step.id,
          ok: true,
          durationMs: stepLatencyMs,
          tokens,
          response:
            SAMPLE_RESPONSES[step.kind] ?? "Step completed successfully.",
        });
      }

      if (!cancelled) {
        send({
          type: "run:summary",
          ok: okCount,
          fail: failCount,
          durationMs: totalMs,
          tokens: totalTokens,
          costUsd: Number((totalTokens / 1000 * 0.002 + okCount * 0.0005).toFixed(4)),
        });
      }
      controller.close();
    },
    cancel() {
      cancelled = true;
    },
  });
}

export async function POST(
  req: NextRequest,
  context: { params: Promise<{ verticalId: string; name: string }> },
) {
  const { verticalId, name } = await context.params;
  let body: RunBody;
  try {
    body = (await req.json()) as RunBody;
  } catch {
    body = { steps: [] };
  }

  const session = await auth();
  if (session?.gatewayToken) {
    const upstream = await streamFromGateway(
      verticalId,
      name,
      body,
      session.gatewayToken,
    );
    if (upstream) return upstream;
  }

  if (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  ) {
    return new Response(
      JSON.stringify({
        error: {
          code: "gateway_unreachable",
          message: "Workflow runtime unreachable.",
        },
      }),
      {
        status: 502,
        headers: { "Content-Type": "application/json" },
      },
    );
  }

  return new Response(buildMockStream(body), {
    status: 200,
    headers: {
      "Content-Type": "application/x-ndjson; charset=utf-8",
      "X-Test-Run-Source": "mock",
      "Cache-Control": "no-store",
    },
  });
}
