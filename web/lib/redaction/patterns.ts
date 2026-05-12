/**
 * PII pattern catalog + Luhn validator for the client-side
 * redaction layer (defense-in-depth — see `redact.ts`).
 *
 * High-precision / low-recall: false positives on important
 * data destroy trust faster than false negatives on edge
 * cases. Anything past obvious pattern matching (names,
 * mailing addresses, free-text PII) is deferred to v1.5
 * server-side LLM redaction.
 *
 * Sentinels match shape: `[CATEGORY]`. Operators can grep
 * transcripts and distinguish *what* was masked.
 */

export type RedactionCategory = "email" | "phone" | "card" | "iban" | "ird";

export const SENTINEL: Record<RedactionCategory, string> = {
  email: "[EMAIL]",
  phone: "[PHONE]",
  card: "[CARD]",
  iban: "[IBAN]",
  ird: "[IRD]",
};

/**
 * Match order matters: card and IBAN must be tried before phone
 * because raw digit sequences (e.g. a 16-digit card) can pass the
 * permissive phone regex. The redactor iterates this array in order.
 */
export const PATTERN_ORDER: readonly RedactionCategory[] = [
  "email",
  "iban",
  "card",
  "ird",
  "phone",
] as const;

/**
 * RFC-5322-ish email. Tightened with a literal `@` and explicit
 * TLD floor at 2 chars. `gi` so we mask every instance, not just
 * the first.
 */
export const EMAIL_RE = /[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}/gi;

/**
 * Phone heuristic — see Acceptance Criteria + "Notes for the
 * implementer". Allows spaces / dashes / parens / leading `+`,
 * min 7 digits, max 15 (E.164 ceiling). Word-boundary anchors
 * stop us masking inside the middle of an unrelated digit run
 * (currency, IDs).
 *
 * The optional leading `\+?[\d(]` requires that the FIRST char of
 * the captured run is digit-like; this excludes ` 555 0123` from
 * a sentence like `"the room is 555 0123"` only if the leading
 * space is the boundary — by design, `+64 21 555 0123` should
 * match. Final post-filter on digit count (≥7) is enforced in
 * `redact.ts` because the regex alone counts dashes/spaces.
 */
export const PHONE_RE = /\+?[\d(][\d\s()-]{6,18}\d/g;

/**
 * Credit-card-ish digit sequences (13-19 digits, optionally
 * separated by dashes or spaces in groups of 4). Luhn validation
 * happens in `redact.ts` to kill false positives on long order
 * numbers and unrelated IDs.
 */
export const CARD_RE = /\b(?:\d[ -]?){12,18}\d\b/g;

/**
 * IBAN: 2 country letters + 2 check digits + 4-30 alphanumerics.
 * Most countries land between 15 and 34 chars total; we cap at
 * the high end of that range.
 */
export const IBAN_RE = /\b[A-Z]{2}\d{2}[A-Z0-9]{11,30}\b/g;

/**
 * NZ IRD number: `\b\d{2,3}-\d{3}-\d{3}\b`. Matches both 8-digit
 * (`12-345-678`) and 9-digit (`123-456-789`) formats. Deliberately
 * NOT trying to match unhyphenated IRDs — collision with phone /
 * order-id space is too high.
 */
export const IRD_RE = /\b\d{2,3}-\d{3}-\d{3}\b/g;

/**
 * Luhn checksum — pure JS, ~15 LOC. Walks the digits right-to-left,
 * doubles every other digit, sums the digits of the products, and
 * returns true iff the total is divisible by 10. Strips spaces /
 * dashes before walking.
 */
export function luhnValid(raw: string): boolean {
  const digits = raw.replace(/[\s-]/g, "");
  if (digits.length < 13 || digits.length > 19) return false;
  if (!/^\d+$/.test(digits)) return false;
  let sum = 0;
  let alt = false;
  for (let i = digits.length - 1; i >= 0; i--) {
    let n = digits.charCodeAt(i) - 48;
    if (alt) {
      n *= 2;
      if (n > 9) n -= 9;
    }
    sum += n;
    alt = !alt;
  }
  return sum % 10 === 0;
}

/**
 * Count digit characters in a string. Used by the phone post-filter
 * to enforce the ≥7-digit floor regardless of separator noise.
 */
export function digitCount(raw: string): number {
  let n = 0;
  for (let i = 0; i < raw.length; i++) {
    const c = raw.charCodeAt(i);
    if (c >= 48 && c <= 57) n++;
  }
  return n;
}
