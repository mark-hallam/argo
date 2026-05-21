/**
 * Time-bound access helpers — surfaces the UX layer for member
 * `expires_at`. Backend contract: a member record may carry
 * `expires_at: string | null` (ISO-8601). When non-null, the member
 * loses access automatically on that date — the nightly auto-revoke
 * gateway job (`mission-gateway-time-bound-access-revoke`,
 * out-of-scope for this brief) flips `revoked_at = expires_at` and
 * invalidates active session cookies.
 *
 * Web is the UX surface: this module owns the pill-tone math
 * (green/amber/red), the human-friendly remaining-time formatter,
 * and the audit-event name catalogue.
 */

export type ExpiryTone = "green" | "amber" | "red";

export interface ExpiryStatus {
  /** Remaining whole days (negative when expired). */
  days: number;
  /** Pill colour band. */
  tone: ExpiryTone;
  /** Whether the expiry is in the past. */
  expired: boolean;
  /** Pill copy — "Expires in 14 days" / "Expired 2 days ago". */
  label: string;
}

const MS_PER_DAY = 24 * 60 * 60 * 1000;

/** Compute pill tone + label from an ISO `expires_at` against `now`. */
export function expiryStatus(expiresAt: string, now: Date = new Date()): ExpiryStatus {
  const t = Date.parse(expiresAt);
  if (Number.isNaN(t)) {
    return { days: 0, tone: "red", expired: true, label: "Expiry unknown" };
  }
  const diffMs = t - now.getTime();
  const days = Math.ceil(diffMs / MS_PER_DAY);
  if (days < 0) {
    const ago = Math.abs(days);
    return {
      days,
      tone: "red",
      expired: true,
      label: ago === 1 ? "Expired 1 day ago" : `Expired ${ago} days ago`,
    };
  }
  if (days <= 7) {
    return {
      days,
      tone: "amber",
      expired: false,
      label:
        days === 0
          ? "Expires today"
          : days === 1
            ? "Expires tomorrow"
            : `Expires in ${days} days`,
    };
  }
  return {
    days,
    tone: "green",
    expired: false,
    label: `Expires in ${days} days`,
  };
}

/** Whether the active viewer should see the imminent-expiry banner. */
export function shouldShowExpiryBanner(
  expiresAt: string | null | undefined,
  now: Date = new Date(),
): boolean {
  if (!expiresAt) return false;
  const status = expiryStatus(expiresAt, now);
  return !status.expired && status.days <= 7;
}

/** Preset radio choices in the invite + edit modals. `custom` opens
 *  the date picker. */
export type ExpiryPreset = "never" | "7d" | "30d" | "90d" | "custom";

export const EXPIRY_PRESET_LABELS: Record<ExpiryPreset, string> = {
  never: "Never",
  "7d": "7 days",
  "30d": "30 days",
  "90d": "90 days",
  custom: "Custom date",
};

export const EXPIRY_PRESETS: readonly ExpiryPreset[] = [
  "never",
  "7d",
  "30d",
  "90d",
  "custom",
] as const;

/** Convert a preset choice into an ISO `expires_at` for `POST/PATCH`.
 *  `custom` requires a `customDate` (yyyy-mm-dd from the picker). */
export function presetToExpiresAt(
  preset: ExpiryPreset,
  customDate: string | null,
  now: Date = new Date(),
): string | null {
  if (preset === "never") return null;
  if (preset === "custom") {
    if (!customDate) return null;
    const t = Date.parse(`${customDate}T23:59:59Z`);
    return Number.isNaN(t) ? null : new Date(t).toISOString();
  }
  const days = preset === "7d" ? 7 : preset === "30d" ? 30 : 90;
  const expires = new Date(now.getTime() + days * MS_PER_DAY);
  return expires.toISOString();
}

/** Default date for the custom-date picker — 30 days from now,
 *  formatted as yyyy-mm-dd for `<input type="date">`. */
export function defaultCustomDate(now: Date = new Date()): string {
  const d = new Date(now.getTime() + 30 * MS_PER_DAY);
  return d.toISOString().slice(0, 10);
}

/** Audit-event names emitted on time-bound member actions. The
 *  gateway is the source of truth for these strings; web surfaces
 *  match them 1:1 in the audit feed. The actual emit lives in the
 *  follow-up `mission-gateway-time-bound-access-revoke` brief; web
 *  exports the names so the eventual audit-row renderer can look up
 *  human copy from a single table. */
export const TIME_BOUND_AUDIT_EVENTS = {
  inviteSent: "member.invite-sent.time-bound",
  accessExtended: "member.access-extended.time-bound",
  accessExpired: "member.access-expired.time-bound",
} as const;

export type TimeBoundAuditEvent =
  (typeof TIME_BOUND_AUDIT_EVENTS)[keyof typeof TIME_BOUND_AUDIT_EVENTS];

export const TIME_BOUND_AUDIT_LABELS: Record<TimeBoundAuditEvent, string> = {
  "member.invite-sent.time-bound": "Invite sent (time-bound)",
  "member.access-extended.time-bound": "Access extended",
  "member.access-expired.time-bound": "Access expired",
};

/** Tooltip copy for the clock icon next to a time-bound member's
 *  name in audit / per-member views. */
export function timeBoundTooltip(
  expiresAt: string | null | undefined,
  now: Date = new Date(),
): string | null {
  if (!expiresAt) return null;
  const status = expiryStatus(expiresAt, now);
  const pretty = prettyDate(expiresAt);
  return status.expired
    ? `Time-bound access — expired ${pretty}`
    : `Time-bound access — expires ${pretty}`;
}

function prettyDate(iso: string): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return iso;
  return new Date(t).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}
