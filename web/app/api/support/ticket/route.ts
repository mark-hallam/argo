import { NextResponse, type NextRequest } from "next/server";
import { auth } from "@/auth";

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

function err(code: string, message: string, status: number) {
  return NextResponse.json({ error: { code, message } }, { status });
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
  const fwd = req.headers.get("x-forwarded-for");
  const payload = {
    email,
    message,
    context: {
      gateway_token: session?.gatewayToken,
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
    if (!upstream.ok) return err("support_upstream", `Support provider returned ${upstream.status}.`, 502);
    const text = await upstream.text();
    let parsed: Record<string, unknown> = {};
    if (text) {
      try {
        const v = JSON.parse(text);
        if (v && typeof v === "object") parsed = v as Record<string, unknown>;
      } catch { /* ignore */ }
    }
    return NextResponse.json({ status: "queued", ...parsed }, { status: 202 });
  } catch {
    return err("support_unreachable", "Couldn't reach support provider.", 502);
  }
}
