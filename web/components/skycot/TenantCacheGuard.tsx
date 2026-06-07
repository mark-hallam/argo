"use client";

import { useQueryClient } from "@tanstack/react-query";
import { useSession } from "next-auth/react";
import { useEffect, useRef } from "react";
import { purgeClientTenantState } from "@/lib/auth/purge-client-state";

/**
 * Invisible guard that purges all client-side tenant state whenever the
 * signed-in principal CHANGES on the same browser — e.g. user B signs in
 * on a shared/handed-off machine where user A's session lingered (A never
 * pressed "sign out", or A's session expired and B logged in).
 *
 * The explicit sign-out path purges immediately (see `UserPill`); this
 * guard is the belt-and-suspenders for the no-explicit-sign-out case. It
 * tracks the LAST non-null principal, so a different next login triggers
 * a purge even across an intermediate logged-out (null) state, while the
 * SAME user re-logging-in (or a token refresh) does not. Initial login
 * (null → A) never purges.
 *
 * Must render inside both `SessionProvider` (for `useSession`) and
 * `QueryProvider` (for `useQueryClient`) — wired in the root layout.
 */
function principalKey(token: string | undefined): string | null {
  if (!token) return null;
  const parts = token.split(".");
  if (parts.length < 2) return null;
  try {
    const b64 = parts[1].replace(/-/g, "+").replace(/_/g, "/");
    const padded = b64 + "=".repeat((4 - (b64.length % 4)) % 4);
    const json =
      typeof atob === "function"
        ? atob(padded)
        : Buffer.from(padded, "base64").toString("utf8");
    const body = JSON.parse(json) as { sub?: string; email?: string; tnt?: string };
    const id = body.email ?? body.sub ?? null;
    return id ? `${id}|${body.tnt ?? ""}` : null;
  } catch {
    return null;
  }
}

export function TenantCacheGuard(): null {
  const { data } = useSession();
  const queryClient = useQueryClient();
  const principal = principalKey(data?.gatewayToken);
  const lastNonNull = useRef<string | null>(null);

  useEffect(() => {
    if (principal === null) return; // logged out: keep lastNonNull for next login
    if (lastNonNull.current !== null && lastNonNull.current !== principal) {
      purgeClientTenantState(queryClient);
    }
    lastNonNull.current = principal;
  }, [principal, queryClient]);

  return null;
}
