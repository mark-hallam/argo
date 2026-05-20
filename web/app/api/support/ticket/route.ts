import { NextResponse, type NextRequest } from "next/server";
import { auth } from "@/auth";

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
const DEFAULT_GATEWAY = "http://127.0.0.1:7400";
const SUBJECT_PREVIEW_CHARS = 120;

function err(code: string, message: string, status: number) {
  return NextResponse.json({ error: { code, message } }, { status });
}

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

function subjectFromMessage(message: string): string {
  const firstLine = message.split(/\r?\n/, 1)[0] ?? "";
  const trimmed = firstLine.trim();
  if (trimmed.length === 0) return "Support request";
  if (trimmed.length <= SUBJECT_PREVIEW_CHARS) return trimmed;
  return `${trimmed.slice(0, SUBJECT_PREVIEW_CHARS - 1)}…`;
}

async function persistTicket(
  email: string,
  message: string,
  gatewayToken: string | undefined,
): Promise<string | undefined> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Accept: "application/json",
  };
  if (gatewayToken) headers.Authorization = `Bearer ${gatewayToken}`;
  // X-Argo-Principal is required by the gateway POST gate even though
  // /api/v1/support/tickets is not admin-gated; pass the submitter
  // identity so audit trail carries it.
  headers["X-Argo-Principal"] = `human:${email}`;
  try {
    const res = await fetch(`${gatewayBase()}/api/v1/support/tickets`, {
      method: "POST",
      headers,
      body: JSON.stringify({
        user_email: email,
        subject: subjectFromMessage(message),
        body: message,
      }),
      cache: "no-store",
    });
    if (!res.ok) return undefined;
    const json = (await res.json().catch(() => ({}))) as { ticket_id?: string };
    return json.ticket_id;
  } catch {
    return undefined;
  }
}

export async function POST(req: NextRequest) {
  let body: { email?: unknown; message?: unknown };
  try {
    body = await req.json();
  } catch {
    return err("invalid_body", "Request body must be JSON.", 400);
  }
  const email = typeof body.email === "string" ? body.email : "";
  const message = typeof body.message === "string" ? body.message : "";
  if (!EMAIL.test(email) || message.trim().length === 0 || message.length > 8000) {
    return err("invalid_ticket", "Email and message are required.", 400);
  }
  const webhookUrl = process.env.SUPPORT_WEBHOOK_URL;
  if (!webhookUrl) return err("support_unconfigured", "Support webhook is not configured.", 503);

  const session = await auth();
  // Persist first so the operator inbox surfaces the ticket even when
  // the outbound webhook fails (claude-support-inbox-spa-page).
  const ticketId = await persistTicket(email, message, session?.gatewayToken);

  const fwd = req.headers.get("x-forwarded-for");
  const payload = {
    email,
    message,
    context: {
      gateway_token: session?.gatewayToken,
      ticket_id: ticketId,
      user_agent: req.headers.get("user-agent") ?? null,
      client_ip: (fwd?.split(",")[0]?.trim() ?? req.headers.get("x-real-ip")) || null,
      submitted_at: new Date().toISOString(),
    },
  };
  const headers: Record<string, string> = { "Content-Type": "application/json", Accept: "application/json" };
  const token = process.env.SUPPORT_WEBHOOK_TOKEN;
  if (token) headers.Authorization = `Bearer ${token}`;
  try {
    const upstream = await fetch(webhookUrl, { method: "POST", headers, body: JSON.stringify(payload), cache: "no-store" });
    if (!upstream.ok) {
      // Webhook delivery failed but the ticket is already persisted —
      // surface 202 with a degraded note so the SPA toast stays positive.
      if (ticketId) {
        return NextResponse.json(
          {
            status: "queued",
            ticket_id: ticketId,
            warning: "support_webhook_failed",
          },
          { status: 202 },
        );
      }
      return err("support_upstream", `Support provider returned ${upstream.status}.`, 502);
    }
    const text = await upstream.text();
    let parsed: Record<string, unknown> = {};
    if (text) {
      try {
        const v = JSON.parse(text);
        if (v && typeof v === "object") parsed = v as Record<string, unknown>;
      } catch { /* ignore */ }
    }
    return NextResponse.json(
      { status: "queued", ticket_id: ticketId, ...parsed },
      { status: 202 },
    );
  } catch {
    if (ticketId) {
      return NextResponse.json(
        {
          status: "queued",
          ticket_id: ticketId,
          warning: "support_webhook_unreachable",
        },
        { status: 202 },
      );
    }
    return err("support_unreachable", "Couldn't reach support provider.", 502);
  }
}
