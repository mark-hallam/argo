"use client";

import { useSession } from "next-auth/react";
import {
  POLICY_MASK_ALL,
  POLICY_SHOW_ALL,
  type RedactionPolicy,
} from "@/lib/redaction/redact";

/**
 * The RBAC permission that grants unredacted view. v1 treats it as a
 * single all-or-nothing flag; v1.5 may split into per-category
 * permissions (`unredacted-email`, `unredacted-phone`, …) — see
 * brief E1-e Open Question #4.
 */
export const UNREDACTED_VIEW_PERMISSION = "unredacted-view";

/**
 * Session augmentation: the NextAuth session may carry an
 * RBAC-derived `permissions` array, populated by the JWT callback
 * once the gateway returns it. The base `auth.config.ts` does not
 * populate this field today (server-side RBAC wiring is the sibling
 * brief J2). Augmenting here keeps forward-compat without forcing a
 * non-may-touch edit on `auth.config.ts`.
 */
declare module "next-auth" {
  interface Session {
    permissions?: string[];
  }
}

function authBypassEnabled(): boolean {
  return process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1";
}

/**
 * Resolve the viewer's effective redaction policy.
 *
 * v1 resolution order:
 *  1. `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` → `POLICY_SHOW_ALL`
 *     (dev/test trusts the operator; matches the rest of the app's
 *     bypass treatment in `lib/api/*`).
 *  2. Session carries `unredacted-view` in `permissions` →
 *     `POLICY_SHOW_ALL`.
 *  3. Otherwise → `POLICY_MASK_ALL` (conservative default).
 *
 * The hook is a thin selector over `useSession()`; the underlying
 * cache lives in the NextAuth `SessionProvider` already mounted at
 * the app root.
 *
 * @example
 *   const policy = useRedactionPolicy();
 *   const { redacted } = redactPII(input, policy);
 */
export function useRedactionPolicy(): RedactionPolicy {
  const { data: session } = useSession();
  if (authBypassEnabled()) return POLICY_SHOW_ALL;
  const perms = session?.permissions ?? [];
  if (perms.includes(UNREDACTED_VIEW_PERMISSION)) return POLICY_SHOW_ALL;
  return POLICY_MASK_ALL;
}
