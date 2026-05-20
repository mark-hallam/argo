import { NextResponse, type NextRequest } from "next/server";
import { auth } from "@/auth";

const DEFAULT_GATEWAY = "http://127.0.0.1:7400";
const SKILL_NAME = "propose_workflow_from_plain_english";
const MAX_INPUT_CHARS = 4000;
const STREAM_CHUNK_DELAY_MS = 28;

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

function jsonError(code: string, message: string, status: number) {
  return NextResponse.json({ error: { code, message } }, { status });
}

interface ProposeBody {
  workerId?: unknown;
  verticalId?: unknown;
  description?: unknown;
}

function pickString(v: unknown, max: number): string {
  return typeof v === "string" ? v.slice(0, max) : "";
}

function snakeCase(input: string): string {
  const slug = input
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
  if (slug.length === 0) return "new_responsibility";
  return slug.length > 48 ? slug.slice(0, 48) : slug;
}

function mockToml(description: string, verticalId: string): string {
  const trimmed = description.trim();
  const head = trimmed.split(/\r?\n/, 1)[0] ?? "";
  const name = snakeCase(head || "new_responsibility");
  const oneLine = trimmed.replace(/[\r\n]+/g, " ").replace(/\s+/g, " ").trim();
  const desc =
    oneLine.length > 200 ? `${oneLine.slice(0, 197)}…` : oneLine || "Proposed workflow.";
  const verticalTag = verticalId ? `# vertical: ${verticalId}\n` : "";
  return `${verticalTag}schema_version = "1"

[workflow]
name        = "${name}"
description = "${desc.replace(/"/g, '\\"')}"

[[step]]
name = "describe_intent"
kind = "prompt"
prompt = "Operator described: ${desc.replace(/"/g, '\\"')}. Confirm the cadence and any approval gates before running."

[[step]]
name = "operator_review"
kind = "prompt"
prompt = "Pause and surface the plan to the operator for approval before any external action."
depends_on = ["describe_intent"]
`;
}

function buildStream(toml: string): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  const chunks = toml.match(/[\s\S]{1,48}/g) ?? [toml];
  let index = 0;
  return new ReadableStream<Uint8Array>({
    async pull(controller) {
      if (index >= chunks.length) {
        controller.close();
        return;
      }
      controller.enqueue(encoder.encode(chunks[index]!));
      index += 1;
      if (index < chunks.length) {
        await new Promise((resolve) => setTimeout(resolve, STREAM_CHUNK_DELAY_MS));
      }
    },
  });
}

async function streamFromGateway(
  description: string,
  verticalId: string,
  workerId: string,
  gatewayToken: string,
): Promise<Response | null> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Accept: "text/plain, application/x-ndjson, */*",
    Authorization: `Bearer ${gatewayToken}`,
  };
  try {
    const upstream = await fetch(`${gatewayBase()}/api/v1/skills/${SKILL_NAME}/invoke/stream`, {
      method: "POST",
      headers,
      body: JSON.stringify({
        worker_id: workerId,
        vertical_id: verticalId,
        input: { description },
        mode: "one_shot",
      }),
      cache: "no-store",
    });
    if (!upstream.ok || !upstream.body) return null;
    return new Response(upstream.body, {
      status: 200,
      headers: { "Content-Type": "text/plain; charset=utf-8", "X-Workflow-Source": "gateway" },
    });
  } catch {
    return null;
  }
}

export async function POST(req: NextRequest) {
  let body: ProposeBody;
  try {
    body = (await req.json()) as ProposeBody;
  } catch {
    return jsonError("invalid_body", "Request body must be JSON.", 400);
  }
  const description = pickString(body.description, MAX_INPUT_CHARS).trim();
  const verticalId = pickString(body.verticalId, 64).trim();
  const workerId = pickString(body.workerId, 64).trim();
  if (description.length === 0) {
    return jsonError("missing_description", "description is required.", 400);
  }
  if (workerId.length === 0) {
    return jsonError("missing_worker_id", "workerId is required.", 400);
  }

  const session = await auth();
  if (session?.gatewayToken) {
    const upstream = await streamFromGateway(
      description,
      verticalId,
      workerId,
      session.gatewayToken,
    );
    if (upstream) return upstream;
  }

  if (process.env.NODE_ENV === "production" && process.env.NEXT_PUBLIC_ENV === "production") {
    return jsonError("gateway_unreachable", "Could not reach workflow-proposal service.", 502);
  }

  const stream = buildStream(mockToml(description, verticalId));
  return new Response(stream, {
    status: 200,
    headers: { "Content-Type": "text/plain; charset=utf-8", "X-Workflow-Source": "mock" },
  });
}
