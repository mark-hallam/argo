"use client";

import { useMemo } from "react";
import { useSession } from "next-auth/react";
import { isDemoMode } from "./demo-mode";

/**
 * Authenticated view-model for the current SPA visitor.
 *
 * Single source of truth for the sidebar UserPill and any other surface
 * that needs the signed-in user's email / name / workspace. Wraps
 * NextAuth's `useSession()` and decodes the gateway JWT body
 * (`session.gatewayToken`) for the `email`, `sub`, `tnt`, and `role`
 * claims. The JWT signature is the gateway's trust boundary; this hook
 * only reads claims for display, never for authorization.
 *
 * When `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, the hook returns the seeded
 * demo identity ("Sam Harlow · Acmeco · Owner") regardless of session
 * state so the v1.0 recorded demo still plays back cleanly.
 */
export interface SkycotSession {
  status: "authenticated" | "unauthenticated" | "loading";
  isDemoMode: boolean;
  email: string | null;
  displayName: string;
  tenantId: string | null;
  workspaceLabel: string;
  role: string | null;
}

interface JwtBody {
  sub?: string;
  email?: string;
  name?: string;
  tnt?: string;
  role?: string;
}

export const DEMO_SESSION: SkycotSession = {
  status: "authenticated",
  isDemoMode: true,
  email: "sam@acmeco.com",
  displayName: "Sam Harlow",
  tenantId: "tenant_acmeco",
  workspaceLabel: "Acmeco · Owner",
  role: "owner",
};

export function useSkycotSession(): SkycotSession {
  const { data, status } = useSession();
  return useMemo(() => deriveSession(data, status), [data, status]);
}

export function deriveSession(
  data: { gatewayToken?: string; role?: string } | null | undefined,
  status: "authenticated" | "unauthenticated" | "loading",
): SkycotSession {
  if (isDemoMode()) return DEMO_SESSION;

  const claims = decodeJwtBody(data?.gatewayToken);
  const email = claims.email ?? null;
  const role = data?.role ?? claims.role ?? null;
  const tenantId = claims.tnt ?? null;

  return {
    status,
    isDemoMode: false,
    email,
    displayName: deriveDisplayName(claims, email),
    tenantId,
    workspaceLabel: deriveWorkspaceLabel(tenantId, role),
    role,
  };
}

function decodeJwtBody(token: string | undefined): JwtBody {
  if (!token) return {};
  const parts = token.split(".");
  if (parts.length < 2) return {};
  try {
    const b64 = parts[1].replace(/-/g, "+").replace(/_/g, "/");
    const padded = b64 + "=".repeat((4 - (b64.length % 4)) % 4);
    const json =
      typeof atob === "function"
        ? atob(padded)
        : Buffer.from(padded, "base64").toString("utf8");
    return JSON.parse(json) as JwtBody;
  } catch {
    return {};
  }
}

function deriveDisplayName(claims: JwtBody, email: string | null): string {
  if (claims.name && claims.name.trim().length > 0) return claims.name.trim();
  if (claims.sub && claims.sub !== email && !claims.sub.includes("@")) {
    return claims.sub;
  }
  if (email) {
    const local = email.split("@")[0] ?? "";
    if (local.length > 0) return local;
  }
  return "Guest";
}

function deriveWorkspaceLabel(
  tenantId: string | null,
  role: string | null,
): string {
  if (!tenantId) return "Personal workspace";
  const slug = tenantId.startsWith("tenant_") ? tenantId.slice(7) : tenantId;
  const pretty =
    slug.length > 0 ? slug.charAt(0).toUpperCase() + slug.slice(1) : tenantId;
  if (role && role.trim().length > 0) {
    const r = role.trim();
    return `${pretty} · ${r.charAt(0).toUpperCase() + r.slice(1)}`;
  }
  return pretty;
}
