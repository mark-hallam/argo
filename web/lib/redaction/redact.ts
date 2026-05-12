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
