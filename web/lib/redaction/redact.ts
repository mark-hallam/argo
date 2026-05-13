/**
 * Client-side PII redaction (defense in depth).
 *
 * **This is NOT the only line of defense.** Real PII protection
 * requires server-side redaction in the gateway; data arriving
 * here is already in browser memory. The companion backend brief
 * `J2-gateway-approvals-pii-redaction-server-side` handles that.
 * This layer guards against scenarios where data was loaded by a
 * higher-privileged session and viewed by a lower-privileged user
 * later (session sharing, screen recording, delegate handoff).
 *
 * High-precision / low-recall: only obvious pattern-matchable PII
 * is masked. Names, mailing addresses, and free-text PII are
 * deferred to v1.5 LLM-based detection.
 *
 * @example
 *   redactPII("contact me at sam@acmeco.com", { email: "mask" })
 *   // => { redacted: "contact me at [EMAIL]", masked: { email: 1, ... } }
 */

import {
  CARD_RE,
  digitCount,
  EMAIL_RE,
  IBAN_RE,
  IRD_RE,
  luhnValid,
  PATTERN_ORDER,
  PHONE_RE,
  SENTINEL,
  type RedactionCategory,
} from "./patterns";
import {
  APPROVER_SENTINEL,
  COUNTERPARTY_SENTINEL,
  redactApproverName,
  redactCounterpartyHandles,
  shouldHideClassification,
  shouldRedactCounterparty,
} from "./audit-rules";
import type { ViewerPolicy } from "./types";
import type { AuditRow } from "@/lib/types/audit";
import type { AuditDetail } from "@/lib/types/audit-detail";

export type RedactionMode = "mask" | "show";

export interface RedactionPolicy {
  email: RedactionMode;
  phone: RedactionMode;
  card: RedactionMode;
  iban: RedactionMode;
  ird: RedactionMode;
}

/** All-mask: the conservative default for a viewer without `unredacted-view`. */
export const POLICY_MASK_ALL: RedactionPolicy = {
  email: "mask",
  phone: "mask",
  card: "mask",
  iban: "mask",
  ird: "mask",
};

/** All-show: the bypass policy for elevated viewers / dev mode. */
export const POLICY_SHOW_ALL: RedactionPolicy = {
  email: "show",
  phone: "show",
  card: "show",
  iban: "show",
  ird: "show",
};

export type CategoryCounts = Record<RedactionCategory, number>;

export interface RedactionResult {
  /** The text after sentinels have been substituted. */
  redacted: string;
  /** Per-category mask count; useful for the "any redaction active?" check. */
  masked: CategoryCounts;
}

const PATTERN: Record<RedactionCategory, RegExp> = {
  email: EMAIL_RE,
  phone: PHONE_RE,
  card: CARD_RE,
  iban: IBAN_RE,
  ird: IRD_RE,
};

/**
 * ISO date pattern (YYYY-MM-DD). The phone regex is permissive enough
 * that 8-digit dates slip through; we filter them out post-hoc so the
 * regex itself stays simple. Anchored on a trimmed match so leading
 * whitespace inside the captured chunk doesn't disrupt the test.
 */
const ISO_DATE_RE = /^\s*\d{4}-\d{2}-\d{2}\s*$/;

/** Per-category post-filter: keeps the regex permissive, kills the obvious false-positives. */
function shouldMask(category: RedactionCategory, match: string): boolean {
  if (category === "phone") {
    if (ISO_DATE_RE.test(match)) return false;
    const n = digitCount(match);
    return n >= 7 && n <= 15;
  }
  if (category === "card") return luhnValid(match);
  return true;
}

/**
 * Walk `PATTERN_ORDER`, substituting each pattern's matches with its
 * sentinel when the policy says `"mask"`. Patterns are applied in a
 * fixed order so the more-specific categories (email, IBAN, card,
 * IRD) consume their digits before the catch-all phone regex sees
 * them.
 *
 * Returns the redacted text plus a per-category count of replacements
 * so callers (e.g. RequestElevatedView visibility) can tell whether
 * anything was masked without re-scanning.
 */
export function redactPII(text: string, policy: RedactionPolicy): RedactionResult {
  let out = text;
  const masked: CategoryCounts = {
    email: 0,
    phone: 0,
    card: 0,
    iban: 0,
    ird: 0,
  };

  for (const category of PATTERN_ORDER) {
    if (policy[category] !== "mask") continue;
    const re = new RegExp(PATTERN[category].source, PATTERN[category].flags);
    out = out.replace(re, (match) => {
      if (!shouldMask(category, match)) return match;
      masked[category]++;
      return SENTINEL[category];
    });
  }

  return { redacted: out, masked };
}

/** True iff `policy` would mask at least one category. */
export function policyMasksAnything(policy: RedactionPolicy): boolean {
  return (Object.values(policy) as RedactionMode[]).some((m) => m === "mask");
}

/** True iff `result.masked` records at least one substitution. */
export function resultHasMasks(result: RedactionResult): boolean {
  return (Object.values(result.masked) as number[]).some((n) => n > 0);
}

/** Categories that were masked at least once in `result`. */
export function maskedCategories(result: RedactionResult): RedactionCategory[] {
  return (Object.keys(result.masked) as RedactionCategory[]).filter(
    (c) => result.masked[c] > 0,
  );
}

/**
 * Run the pattern pass + the counterparty pass (when permitted by
 * the viewer's policy). Audit-only consumers; approval-inbox stays
 * on `redactPII` to keep its behavior identical to E1-e.
 */
function redactAuditString(text: string, policy: ViewerPolicy): string {
  const { redacted } = redactPII(text, policy);
  if (!shouldRedactCounterparty(policy.permissions)) return redacted;
  return redactCounterpartyHandles(redacted).redacted;
}

/** Recursively redact string values inside an unknown JSON-ish payload. */
function redactAuditValue(value: unknown, policy: ViewerPolicy): unknown {
  if (typeof value === "string") return redactAuditString(value, policy);
  if (Array.isArray(value)) {
    return value.map((item) => redactAuditValue(item, policy));
  }
  if (value !== null && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      out[k] = redactAuditValue(v, policy);
    }
    return out;
  }
  return value;
}

/**
 * RBAC-aware audit-row redaction. Applies PII patterns to `verb`,
 * masks the approver name when the viewer lacks `unredactedApprover`,
 * drops `dataClassification` when masked, and rewrites counterparty
 * handles inside the verb.
 *
 * Pure function; returns a new row. The brief's `(row, policy,
 * viewerRole)` signature collapses to `(row, policy)` because
 * `ViewerPolicy` already carries the role — see PR "Sensible
 * deviations".
 */
export function redactAuditRow(row: AuditRow, policy: ViewerPolicy): AuditRow {
  const verb = redactAuditString(row.verb, policy);
  const approver = row.approver
    ? { ...row.approver, name: redactApproverName(row.approver.name, policy.permissions) }
    : row.approver;
  const dataClassification = shouldHideClassification(policy.permissions)
    ? undefined
    : row.dataClassification;
  return { ...row, verb, approver, dataClassification };
}

/**
 * RBAC-aware audit-detail redaction. Extends `redactAuditRow` to the
 * detail-page payload: inputs.triggerSummary, every decisionChain
 * body, the output.body (string or recursive object), and the
 * follow-up verb. `provenance` is left untouched (hashes aren't
 * PII).
 */
export function redactAuditDetail(detail: AuditDetail, policy: ViewerPolicy): AuditDetail {
  const baseRow = redactAuditRow(detail, policy);
  const inputs = {
    ...detail.inputs,
    triggerSummary: redactAuditString(detail.inputs.triggerSummary, policy),
    predecessor: detail.inputs.predecessor
      ? { ...detail.inputs.predecessor, verb: redactAuditString(detail.inputs.predecessor.verb, policy) }
      : detail.inputs.predecessor,
  };
  const decisionChain = detail.decisionChain.map((step) => ({
    ...step,
    body: redactAuditString(step.body, policy),
  }));
  const output = {
    ...detail.output,
    body:
      typeof detail.output.body === "string"
        ? redactAuditString(detail.output.body, policy)
        : (redactAuditValue(detail.output.body, policy) as Record<string, unknown>),
    followUp: detail.output.followUp
      ? { ...detail.output.followUp, verb: redactAuditString(detail.output.followUp.verb, policy) }
      : detail.output.followUp,
  };
  return {
    ...detail,
    ...baseRow,
    inputs,
    decisionChain,
    output,
  };
}

/** All sentinels (pattern + audit-specific) — exposed so renderers
 *  can detect any masked span without re-importing both modules. */
export const ALL_SENTINELS: readonly string[] = [
  ...(Object.values(SENTINEL) as string[]),
  APPROVER_SENTINEL,
  COUNTERPARTY_SENTINEL,
] as const;

/** True iff `text` contains at least one redaction sentinel. */
export function textHasRedaction(text: string): boolean {
  return ALL_SENTINELS.some((s) => text.includes(s));
}
