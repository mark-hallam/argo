"use client";

import { useCallback, useEffect, useState } from "react";
import type { ElevatedGrant, RbacPermission } from "@/lib/redaction/types";

/**
 * Manages the client-side, time-bounded elevated-view grants for the
 * audit surface. v1 is mock: grants are issued locally on submit,
 * persisted to `localStorage` with a TTL, and revoked on expiry.
 * v1.5 (`mission-enterprise-elevated-view-approval`) replaces the
 * mock with a server-issued cookie-bound grant + operator approval
 * queue.
 *
 * The hook returns the currently-live grants (already filtered for
 * expiry) plus an `issueGrant()` callback for the request dialog.
 * Storage events are listened to so multiple tabs share state.
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

function makeRequestId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `grant-${Math.random().toString(36).slice(2)}-${Date.now().toString(36)}`;
}

export function useElevatedViewGrants(): UseElevatedViewGrantsResult {
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

  const issueGrant = useCallback((input: IssueGrantInput): ElevatedGrant => {
    const grantedAt = nowIso();
    const expiresAt = new Date(Date.now() + GRANT_DURATION_MS[input.duration]).toISOString();
    const grant: ElevatedGrant = {
      grantedAt,
      expiresAt,
      categories: [...input.categories],
      reason: input.reason,
      requestId: input.requestId ?? makeRequestId(),
    };
    setGrants((prev) => {
      const live = prev.filter((g) => isLive(g, Date.now()));
      const next = [...live, grant];
      writeStored(next);
      return next;
    });
    return grant;
  }, []);

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

  return { activeGrants: grants, issueGrant, revokeGrant, clearAll };
}
