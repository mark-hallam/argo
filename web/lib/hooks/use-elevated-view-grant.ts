"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type { ElevatedGrant, RbacPermission } from "@/lib/redaction/types";
import { useSkycotSession } from "@/lib/skycot/use-session";

/**
 * Manages the client-side, time-bounded elevated-view grants for the
 * audit surface. v1 is mock: grants are issued locally on submit,
 * persisted to `localStorage` with a TTL, and revoked on expiry.
 * v1.5 (`mission-enterprise-elevated-view-approval`) replaces the
 * mock with a server-issued cookie-bound grant + operator approval
 * queue.
 *
 * The hook returns the currently-live grants (already filtered for
 * expiry AND for principal/tenant binding) plus an `issueGrant()`
 * callback for the request dialog. Storage events are listened to so
 * multiple tabs share state.
 *
 * **Security — bind on read.** Grants are stamped with the issuing
 * `principal` + `tenantId` and, on every read, a grant whose
 * principal+tenant does not match the CURRENT session is ignored. So a
 * PII unlock minted under user/tenant A is inert under user/tenant B
 * even if its bytes survive in `localStorage` (the identity-change purge
 * also clears them, but bind-on-read makes a survivor harmless on its
 * own). Legacy grants with no principal/tenant (minted before this
 * binding existed) never match a real session → ignored, fail-safe.
 */

const STORAGE_KEY = "skycot.elevated-grants";

export const GRANT_DURATION_MS: Record<GrantDuration, number> = {
  "1h": 60 * 60_000,
  "4h": 4 * 60 * 60_000,
  "8h": 8 * 60 * 60_000,
  "24h": 24 * 60 * 60_000,
};

export type GrantDuration = "1h" | "4h" | "8h" | "24h";

export const GRANT_DURATION_LABELS: Record<GrantDuration, string> = {
  "1h": "1 hour",
  "4h": "4 hours",
  "8h": "8 hours",
  "24h": "24 hours",
};

export interface IssueGrantInput {
  categories: RbacPermission[];
  reason: string;
  duration: GrantDuration;
  /** Optional override for the request id (tests). */
  requestId?: string;
}

/**
 * The identity a grant is bound to. A grant is only honored when BOTH
 * fields match the grant's stamped `principal` + `tenantId`. `null`
 * means "no resolved identity" (logged out / loading), which matches
 * nothing — so no grant is active without a concrete session.
 */
export interface GrantSessionBinding {
  principal: string | null;
  tenantId: string | null;
}

export interface UseElevatedViewGrantsResult {
  activeGrants: ElevatedGrant[];
  issueGrant: (input: IssueGrantInput) => ElevatedGrant;
  revokeGrant: (requestId: string) => void;
  clearAll: () => void;
}

function nowIso(): string {
  return new Date().toISOString();
}

function readStored(): ElevatedGrant[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isGrant);
  } catch {
    return [];
  }
}

function writeStored(grants: ElevatedGrant[]): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(grants));
  } catch {
    /* quota / private-mode → silently drop */
  }
}

function isGrant(value: unknown): value is ElevatedGrant {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.grantedAt === "string" &&
    typeof v.expiresAt === "string" &&
    Array.isArray(v.categories) &&
    typeof v.reason === "string" &&
    typeof v.requestId === "string"
  );
}

function isLive(grant: ElevatedGrant, now: number): boolean {
  const expiry = Date.parse(grant.expiresAt);
  if (Number.isNaN(expiry)) return false;
  return expiry > now;
}

/**
 * Fail-safe binding check: a grant is honored only when its stamped
 * `principal` AND `tenantId` exactly match the current session's. A
 * grant missing either field (legacy, pre-binding) never matches a
 * concrete session, and a `null` session field (logged out / loading)
 * matches nothing. So a grant minted under identity A is never active
 * under identity B.
 *
 * Exported pure so tests can drive it without a session provider.
 */
export function grantMatchesBinding(
  grant: ElevatedGrant,
  binding: GrantSessionBinding,
): boolean {
  if (binding.principal === null || binding.tenantId === null) return false;
  return (
    grant.principal === binding.principal &&
    grant.tenantId === binding.tenantId
  );
}

/** Filter stored grants down to those that are both unexpired AND bound
 *  to the current session. The single chokepoint every consumer reads
 *  through. Exported pure for tests. */
export function liveGrantsForSession(
  grants: ElevatedGrant[],
  binding: GrantSessionBinding,
  now: number,
): ElevatedGrant[] {
  return grants.filter(
    (g) => isLive(g, now) && grantMatchesBinding(g, binding),
  );
}

function makeRequestId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `grant-${Math.random().toString(36).slice(2)}-${Date.now().toString(36)}`;
}

export function useElevatedViewGrants(): UseElevatedViewGrantsResult {
  const { email, tenantId } = useSkycotSession();
  const [grants, setGrants] = useState<ElevatedGrant[]>(() => readStored());

  useEffect(() => {
    const handle = window.setInterval(() => {
      setGrants((prev) => {
        const now = Date.now();
        const live = prev.filter((g) => isLive(g, now));
        if (live.length !== prev.length) writeStored(live);
        return live.length === prev.length ? prev : live;
      });
    }, 30_000);
    const onStorage = (e: StorageEvent) => {
      if (e.key === STORAGE_KEY) setGrants(readStored());
    };
    window.addEventListener("storage", onStorage);
    return () => {
      window.clearInterval(handle);
      window.removeEventListener("storage", onStorage);
    };
  }, []);

  const issueGrant = useCallback(
    (input: IssueGrantInput): ElevatedGrant => {
      const grantedAt = nowIso();
      const expiresAt = new Date(
        Date.now() + GRANT_DURATION_MS[input.duration],
      ).toISOString();
      const grant: ElevatedGrant = {
        grantedAt,
        expiresAt,
        categories: [...input.categories],
        reason: input.reason,
        requestId: input.requestId ?? makeRequestId(),
        // Bind the grant to whoever is signed in right now. Read on
        // every consumer so a grant is inert outside this identity.
        principal: email ?? undefined,
        tenantId: tenantId ?? undefined,
      };
      setGrants((prev) => {
        const live = prev.filter((g) => isLive(g, Date.now()));
        const next = [...live, grant];
        writeStored(next);
        return next;
      });
      return grant;
    },
    [email, tenantId],
  );

  const revokeGrant = useCallback((requestId: string) => {
    setGrants((prev) => {
      const next = prev.filter((g) => g.requestId !== requestId);
      writeStored(next);
      return next;
    });
  }, []);

  const clearAll = useCallback(() => {
    setGrants([]);
    writeStored([]);
  }, []);

  // Expose only grants that are both unexpired AND bound to the current
  // session. A grant minted under another principal/tenant is filtered
  // out here even if it still sits in `localStorage` — the fail-safe
  // bind-on-read that complements the identity-change purge.
  const activeGrants = useMemo(
    () => liveGrantsForSession(grants, { principal: email, tenantId }, Date.now()),
    [grants, email, tenantId],
  );

  return { activeGrants, issueGrant, revokeGrant, clearAll };
}
