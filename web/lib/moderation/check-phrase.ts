import { BLOCKED_PHRASE_TOKENS } from "./phrase-wordlist";

/**
 * Result of a signature-phrase moderation check. `ok=true` permits
 * the phrase through; `ok=false` carries a short reason suitable for
 * a toast.
 */
export type PhraseCheck =
  | { ok: true }
  | { ok: false; reason: string };

const MIN_PHRASE_LEN = 3;

/**
 * Client-side moderation for a single signature phrase. Filters:
 *  - too short (< 3 chars)
 *  - contains any blocked substring (case-insensitive)
 *
 * Server-side LLM moderation (`mission-enterprise-voice-profile-
 * moderation`) is the architectural target; this surface exists for
 * immediate feedback in the editor.
 */
export function isPhraseAcceptable(phrase: string): PhraseCheck {
  const trimmed = phrase.trim();
  if (trimmed.length < MIN_PHRASE_LEN) {
    return { ok: false, reason: "Phrase is too short." };
  }
  const lower = trimmed.toLowerCase();
  for (const token of BLOCKED_PHRASE_TOKENS) {
    if (lower.includes(token)) {
      return {
        ok: false,
        reason: "That phrase isn't allowed in your teammate's voice — try another.",
      };
    }
  }
  return { ok: true };
}
