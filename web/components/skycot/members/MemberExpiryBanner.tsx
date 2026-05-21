"use client";

import { useEffect, useState } from "react";
import { useSession } from "next-auth/react";
import { Clock, X } from "lucide-react";
import { expiryStatus, shouldShowExpiryBanner } from "@/lib/members/expiry";

/**
 * Slim amber expiry banner mounted at the top of every `(app)` route.
 * Renders only when the active viewer has an `expires_at` within 7
 * days (and not in the past). Dismiss persists per-day via
 * localStorage so the banner doesn't re-show on every navigation
 * within the same day, but re-asserts itself daily as the deadline
 * gets closer.
 *
 * Source of `expires_at`: the NextAuth session (type-augmented
 * below). The gateway populates `session.expiresAtOverride` from
 * the member record's `expires_at` column on JWT mint; until then,
 * the banner stays inert (null → no render).
 */
const STORAGE_KEY = "skycot.expiry-banner.dismissed";

export function MemberExpiryBanner() {
  const { data: session } = useSession();
  const expiresAt = session?.expiresAtOverride ?? null;
  const [dismissedKey, setDismissedKey] = useState<string | null>(null);

  useEffect(() => {
    if (typeof window === "undefined") return;
    try {
      setDismissedKey(window.localStorage.getItem(STORAGE_KEY));
    } catch {
      setDismissedKey(null);
    }
  }, []);

  if (!shouldShowExpiryBanner(expiresAt)) return null;
  const todayKey = todayKeyFor(expiresAt!);
  if (dismissedKey === todayKey) return null;

  const status = expiryStatus(expiresAt!);
  const dayWord = status.days === 1 ? "day" : "days";
  const message =
    status.days === 0
      ? "Your access expires today. Ask your admin to extend if needed."
      : `Your access expires in ${status.days} ${dayWord}. Ask your admin to extend if needed.`;

  function dismiss() {
    setDismissedKey(todayKey);
    try {
      window.localStorage.setItem(STORAGE_KEY, todayKey);
    } catch {
      /* swallow — quota / private mode */
    }
  }

  return (
    <div
      role="status"
      data-testid="expiry-banner"
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        padding: "10px 16px",
        background: "rgba(245, 158, 11, 0.12)",
        color: "#b45309",
        borderBottom: "1px solid rgba(245, 158, 11, 0.25)",
        fontSize: 13,
        fontWeight: 600,
      }}
    >
      <Clock size={14} aria-hidden />
      <span style={{ flex: 1 }}>{message}</span>
      <button
        type="button"
        onClick={dismiss}
        aria-label="Dismiss expiry banner"
        data-testid="expiry-banner-dismiss"
        style={{
          background: "transparent",
          border: "none",
          color: "inherit",
          cursor: "pointer",
          padding: 4,
          display: "grid",
          placeItems: "center",
          borderRadius: 6,
        }}
      >
        <X size={14} />
      </button>
    </div>
  );
}

function todayKeyFor(expiresAt: string): string {
  const today = new Date().toISOString().slice(0, 10);
  return `${today}@${expiresAt}`;
}

declare module "next-auth" {
  interface Session {
    /** ISO `expires_at` for time-bound viewers (auditor / contractor
     *  / external consultant). Null for permanent staff. Populated
     *  by the gateway JWT callback in a sibling brief; until then,
     *  the banner stays inert. */
    expiresAtOverride?: string | null;
  }
}
