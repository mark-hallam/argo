"use client";

import { useMemo } from "react";
import { useSession } from "next-auth/react";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
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
 * When `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, the hook returns a session
 * derived from the persisted onboarding draft so the workspace pill,
 * email, and tenant id reflect whatever the operator typed in step 2.
 * Pre-step-2 (empty draft) renders the neutral
 * `DEMO_SESSION_FALLBACK` framing — never the old hardcoded
 * "Sam Harlow · Acmeco · Owner" identity that contradicted typed
 * inputs on every in-app surface the camera reads.
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

/**
 * Snapshot of the onboarding draft fields that feed demo-mode session
 * derivation. Decoupled from the full draft type so tests + non-React
 * callers can build one without instantiating the zustand store.
 */
export interface DemoDraftSnapshot {
  workspaceName: string;
  tenantId: string | null;
}

/**
 * Neutral demo-mode identity rendered before the onboarding wizard
 * collects a workspace name (and as the fallback when the draft is
 * reset). Owner-role is preserved so RBAC-conditioned chrome still
 * matches what the demo narrative implies.
 */
export const DEMO_SESSION_FALLBACK: SkycotSession = {
  status: "authenticated",
  isDemoMode: true,
  email: "operator@demo.skycot.local",
  displayName: "Demo Operator",
  tenantId: "tenant_demo",
  workspaceLabel: "Sample workspace · Owner",
  role: "owner",
};

export function useSkycotSession(): SkycotSession {
  const { data, status } = useSession();
  // Subscribe to draft fields individually so each yields a primitive
  // — zustand only re-renders when the primitive changes. Non-demo
  // sessions ignore these values in `deriveSession`; the subscription
  // cost is a render-time selector call, not a re-render trigger.
  const workspaceName = useOnboardingDraft((s) => s.workspaceName);
  const tenantId = useOnboardingDraft((s) => s.tenantId);
  return useMemo(
    () => deriveSession(data, status, { workspaceName, tenantId }),
    [data, status, workspaceName, tenantId],
  );
}

export function deriveSession(
  data: { gatewayToken?: string; role?: string } | null | undefined,
  status: "authenticated" | "unauthenticated" | "loading",
  demoDraft: DemoDraftSnapshot | null = null,
): SkycotSession {
  if (isDemoMode()) return getDemoSession(demoDraft);

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

export function getDemoSession(
  draft: DemoDraftSnapshot | null,
): SkycotSession {
  const wsName = draft?.workspaceName?.trim() ?? "";
  if (wsName.length === 0) return DEMO_SESSION_FALLBACK;
  const slug = slugifyWorkspace(wsName);
  return {
    status: "authenticated",
    isDemoMode: true,
    email: `${slug}@demo.skycot.local`,
    displayName: "Demo Operator",
    tenantId: draft?.tenantId ?? `tenant_${slug}`,
    workspaceLabel: `${wsName} · Owner`,
    role: "owner",
  };
}

function slugifyWorkspace(input: string): string {
  const slug = input
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "")
    .slice(0, 40);
  return slug.length > 0 ? slug : "workspace";
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
