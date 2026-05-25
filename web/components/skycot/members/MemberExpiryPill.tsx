"use client";

import { Clock } from "lucide-react";
import { expiryStatus } from "@/lib/members/expiry";

const EXPIRY_DATE_FORMATTER = new Intl.DateTimeFormat("en-AU", {
  day: "numeric",
  month: "short",
  year: "numeric",
  timeZone: "UTC",
});

/** Compact pill rendered next to a member's name/email when the
 *  member has an `expires_at`. Tone tracks time-remaining bands:
 *  green (>7d), amber (≤7d), red (past). Permanent members render
 *  nothing. */
export function MemberExpiryPill({
  expiresAt,
  now,
}: {
  expiresAt: string | null | undefined;
  now?: Date;
}) {
  if (!expiresAt) return null;
  const status = expiryStatus(expiresAt, now);
  return (
    <span
      className="member-expiry-pill"
      data-tone={status.tone}
      data-testid="member-expiry-pill"
      title={`Access ${status.expired ? "expired" : "expires"} on ${formatDate(expiresAt)}`}
    >
      <Clock size={10} aria-hidden /> {status.label}
    </span>
  );
}

function formatDate(iso: string): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return iso;
  return EXPIRY_DATE_FORMATTER.format(new Date(t));
}
