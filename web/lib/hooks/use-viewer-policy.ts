"use client";

import { useMemo } from "react";
import { useSession } from "next-auth/react";
import { useElevatedViewGrants } from "./use-elevated-view-grant";
import {
  ALL_PERMISSIONS,
  PERMISSION_KEY,
  RBAC_PERMISSIONS,
  ROLE_DEFAULT_PERMISSIONS,
  VIEWER_POLICY_DELEGATOR,
  VIEWER_POLICY_OPERATOR,
  VIEWER_ROLES,
  policyFromPermissions,
  type ElevatedGrant,
  type RbacPermission,
  type ViewerPermissions,
  type ViewerPolicy,
  type ViewerRole,
} from "@/lib/redaction/types";

/**
 * Resolve the viewer's effective `ViewerPolicy` for an audit surface.
 *
 * Resolution order:
 *  1. `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` → `VIEWER_POLICY_OPERATOR`
 *     (dev / Storybook bypass — matches the rest of the app).
 *  2. Session carries `unredacted-view` → full operator policy.
 *  3. Session role + per-permission grants from `permissions[]` →
 *     role defaults merged with explicit `unredacted-*` strings.
 *  4. Active elevated-view grants from `useElevatedViewGrants` → flip
 *     the requested categories on for the grant TTL.
 *  5. Default → `VIEWER_POLICY_DELEGATOR` (all-mask).
 *
 * Like E1-e's `useRedactionPolicy`, this is a thin selector — the
 * underlying caches (NextAuth session, localStorage grants) live
 * elsewhere.
 */
export function useViewerPolicy(): ViewerPolicy {
  const { data: session } = useSession();
  const { activeGrants } = useElevatedViewGrants();

  return useMemo(
    () => resolveViewerPolicy(session?.permissions ?? [], session?.role, activeGrants),
    [session?.permissions, session?.role, activeGrants],
  );
}

/** Pure resolver — exported so tests can drive it without a session
 *  provider. Honors the env-bypass via a process.env check, which
 *  vitest can override per-test. */
export function resolveViewerPolicy(
  permissions: string[],
  rawRole: string | undefined,
  grants: ElevatedGrant[],
): ViewerPolicy {
  if (authBypassEnabled()) return VIEWER_POLICY_OPERATOR;

  if (permissions.includes("unredacted-view")) {
    return { ...VIEWER_POLICY_OPERATOR, activeGrants: grants };
  }

  const viewerRole = normalizeRole(rawRole);
  const basePerms: ViewerPermissions = { ...ROLE_DEFAULT_PERMISSIONS[viewerRole] };

  for (const perm of permissions) {
    if ((RBAC_PERMISSIONS as readonly string[]).includes(perm)) {
      basePerms[PERMISSION_KEY[perm as RbacPermission]] = true;
    }
  }

  const withGrants = applyGrants(basePerms, grants);
  const policy: ViewerPolicy = {
    ...policyFromPermissions(withGrants),
    viewerRole,
    permissions: withGrants,
    activeGrants: grants,
  };
  if (viewerRole === "delegator" && grants.length === 0 && permissions.length === 0) {
    return VIEWER_POLICY_DELEGATOR;
  }
  return policy;
}

function applyGrants(
  perms: ViewerPermissions,
  grants: ElevatedGrant[],
): ViewerPermissions {
  if (grants.length === 0) return perms;
  const merged: ViewerPermissions = { ...perms };
  for (const grant of grants) {
    for (const cat of grant.categories) {
      merged[PERMISSION_KEY[cat]] = true;
    }
  }
  return merged;
}

function authBypassEnabled(): boolean {
  return process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1";
}

function normalizeRole(raw: string | undefined): ViewerRole {
  if (raw && (VIEWER_ROLES as readonly string[]).includes(raw)) {
    return raw as ViewerRole;
  }
  return "delegator";
}

/** Session augmentation: NextAuth's session may carry a role string
 *  + permissions array, populated by the gateway JWT callback. The
 *  base `auth.config.ts` doesn't populate these today (sibling
 *  backend brief). Augmenting here keeps forward-compat without
 *  forcing a non-may-touch edit on `auth.config.ts`. */
declare module "next-auth" {
  interface Session {
    role?: string;
    permissions?: string[];
  }
}

export { ALL_PERMISSIONS };
