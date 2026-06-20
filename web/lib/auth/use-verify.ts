"use client";

import { useEffect, useState } from "react";
import { signIn } from "next-auth/react";
import { verifySchema } from "./signup-schema";

export type VerifyState =
  | { kind: "idle" }
  | { kind: "verifying" }
  | { kind: "verified"; needsOnboarding: boolean; tenantId: string | null }
  | { kind: "error"; code: VerifyErrorCode; message: string };

export type VerifyErrorCode =
  | "missing_token"
  | "invalid_token"
  | "expired"
  | "used"
  | "network"
  | "server";

interface VerifyBody {
  needs_onboarding?: boolean;
  tenant_id?: string | null;
  gateway_token?: string | null;
  error?: { code: string; message: string };
}

function classify(code: string | undefined, status: number): VerifyErrorCode {
  if (code === "token_expired" || status === 410) return "expired";
  if (code === "token_used") return "used";
  if (status === 401 || status === 400) return "invalid_token";
  return "server";
}

export function useVerify(token: string | null) {
  const [state, setState] = useState<VerifyState>({ kind: "idle" });

  useEffect(() => {
    let cancelled = false;
    const parsed = verifySchema.safeParse({ token: token ?? "" });
    if (!parsed.success) {
      setState({ kind: "error", code: "missing_token", message: "Missing or malformed magic-link token." });
      return;
    }
    setState({ kind: "verifying" });
    fetch("/api/auth/verify", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ token: parsed.data.token }),
    })
      .then(async (res) => {
        const body = (await res.json().catch(() => ({}))) as VerifyBody;
        if (cancelled) return;
        if (!res.ok) {
          setState({
            kind: "error",
            code: classify(body.error?.code, res.status),
            message: body.error?.message ?? "Sign-in link is no longer valid.",
          });
          return;
        }
        // Establish the NextAuth client session so `useSession()` /
        // `useSkycotSession()` surface `gatewayToken` (and the `tnt`,
        // `email`, `name` claims decoded from it). Without this handoff the
        // `__Host-session` cookie is set but the client session stays
        // unauthenticated, so the app shell falls back to "Guest / Personal
        // workspace". Best-effort: a failed sign-in still leaves the cookie
        // in place (API calls work), so we proceed to the verified state.
        if (body.gateway_token) {
          try {
            await signIn("gateway-bearer", {
              token: body.gateway_token,
              redirect: false,
            });
          } catch {
            // Non-fatal — only the client-side identity display degrades.
          }
        }
        if (cancelled) return;
        setState({
          kind: "verified",
          needsOnboarding: body.needs_onboarding ?? true,
          tenantId: body.tenant_id ?? null,
        });
      })
      .catch(() => {
        if (!cancelled) {
          setState({ kind: "error", code: "network", message: "Couldn't reach SKYCOT. Try the link again in a moment." });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token]);

  return state;
}
