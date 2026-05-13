/**
 * Audit-surface redaction rules. Sits on top of the E1-e pattern
 * primitives (`redactPII`, sentinels) and adds the audit-specific
 * cases the brief calls out:
 *
 *  - Approver names — replaced with `[REDACTED-APPROVER]` when the
 *    viewer lacks `unredacted-approver`; role-only display is left
 *    to the renderer (e.g. "approved by Operator").
 *  - Data classification — hidden (set to `undefined`) when the
 *    viewer lacks `unredacted-classification`. The brief opts out
 *    of a sentinel here so the chip simply doesn't render rather
 *    than showing a placeholder.
 *  - Counterparty handles — `@diego.alvarez`, `vendor#acme`, etc.
 *    Masked with `[REDACTED-COUNTERPARTY]` when the viewer lacks
 *    `unredacted-counterparty`.
 *
 * Pure functions; no React. Consumed by `redactAuditRow` /
 * `redactAuditDetail` in `redact.ts`.
 */

import type { ViewerPermissions } from "./types";

export const APPROVER_SENTINEL = "[REDACTED-APPROVER]";
export const COUNTERPARTY_SENTINEL = "[REDACTED-COUNTERPARTY]";

/** Strict handle: `@` or `#` or `vendor#` followed by 2+ identifier
 *  chars. Word-bounded so prose isn't accidentally chewed. Case-
 *  insensitive because handles regularly mix case (`@DiegoA`). */
const COUNTERPARTY_HANDLE_RE = /(?:^|[\s(\[])([@#][a-z0-9._-]{2,}|[a-z][a-z0-9_-]+#[a-z0-9._-]{2,})/gi;

/** Mask counterparty handles in free-form text. Returns the rewritten
 *  string and a count of substitutions so callers can tell whether the
 *  banner needs to surface a counterparty-redaction note. */
export function redactCounterpartyHandles(text: string): {
  redacted: string;
  count: number;
} {
  let count = 0;
  const redacted = text.replace(COUNTERPARTY_HANDLE_RE, (match, handle: string) => {
    count++;
    const prefix = match.slice(0, match.length - handle.length);
    return `${prefix}${COUNTERPARTY_SENTINEL}`;
  });
  return { redacted, count };
}

/** Decide whether the approver name needs masking. */
export function shouldRedactApprover(perms: ViewerPermissions): boolean {
  return !perms.unredactedApprover;
}

/** Decide whether the data-classification chip needs hiding. */
export function shouldHideClassification(perms: ViewerPermissions): boolean {
  return !perms.unredactedClassification;
}

/** Decide whether counterparty handles need masking. */
export function shouldRedactCounterparty(perms: ViewerPermissions): boolean {
  return !perms.unredactedCounterparty;
}

/** Apply approver masking to a name. Returns the sentinel when masking
 *  is in force; otherwise returns the original. */
export function redactApproverName(name: string, perms: ViewerPermissions): string {
  return shouldRedactApprover(perms) ? APPROVER_SENTINEL : name;
}

/** Apply counterparty masking to text. No-op when the viewer is
 *  permitted to see counterparties. */
export function maybeRedactCounterparty(
  text: string,
  perms: ViewerPermissions,
): { redacted: string; count: number } {
  if (!shouldRedactCounterparty(perms)) return { redacted: text, count: 0 };
  return redactCounterpartyHandles(text);
}

/** Audit-specific sentinels (in addition to the E1-e pattern sentinels)
 *  the banner / dialog need to enumerate. Order matches the categories
 *  checkbox in `<ElevatedViewRequestDialog>`. */
export const AUDIT_SENTINELS: readonly string[] = [
  APPROVER_SENTINEL,
  COUNTERPARTY_SENTINEL,
] as const;

/** Permission keys the dialog can request unlocks for. Drives the
 *  checkbox set; keep order stable so e2e selectors are deterministic. */
export const REQUESTABLE_CATEGORIES: ReadonlyArray<{
  permission: import("./types").RbacPermission;
  label: string;
}> = [
  { permission: "unredacted-email", label: "Email addresses" },
  { permission: "unredacted-phone", label: "Phone numbers" },
  { permission: "unredacted-card", label: "Card numbers" },
  { permission: "unredacted-iban", label: "Bank account / IBAN" },
  { permission: "unredacted-ird", label: "Tax IDs (IRD)" },
  { permission: "unredacted-classification", label: "Data classification" },
  { permission: "unredacted-approver", label: "Approver names" },
  { permission: "unredacted-counterparty", label: "Counterparty handles" },
] as const;

/** Categories the viewer LACKS — the banner pre-selects these in the
 *  request dialog so the user only has to write the reason + duration. */
export function categoriesLackedBy(
  perms: import("./types").ViewerPermissions,
): import("./types").RbacPermission[] {
  return REQUESTABLE_CATEGORIES.flatMap(({ permission }) => {
    const key = (
      {
        "unredacted-email": "unredactedEmail",
        "unredacted-phone": "unredactedPhone",
        "unredacted-card": "unredactedCard",
        "unredacted-iban": "unredactedIBAN",
        "unredacted-ird": "unredactedIRD",
        "unredacted-classification": "unredactedClassification",
        "unredacted-approver": "unredactedApprover",
        "unredacted-counterparty": "unredactedCounterparty",
      } as const
    )[permission];
    return perms[key] ? [] : [permission];
  });
}
