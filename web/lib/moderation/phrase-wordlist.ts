/**
 * Conservative wordlist of obviously-blocked phrase tokens for the
 * Step 4 structured Adjust drawer. Substring match (case-insensitive)
 * is the v1 filter; LLM moderation replaces this surface under
 * `mission-enterprise-voice-profile-moderation`.
 *
 * Tokens are mild category placeholders kept terse on purpose — the
 * goal is to block obvious slurs, sexual content, and violent
 * threats from leaking into a worker's voice profile without
 * spelling them out in the source. Operators with stricter brand-
 * safety needs configure the LLM moderator instead.
 */
const RAW_TOKENS = [
  // generic profanity
  "fuck",
  "shit",
  "bitch",
  "bastard",
  "asshole",
  "dickhead",
  "cunt",
  "piss",
  "cock",
  "wank",
  "twat",
  "bollocks",
  "bullshit",
  "motherfucker",
  // slurs (mild category placeholders; full list maintained server-side)
  "slur1",
  "slur2",
  "slur3",
  "slur4",
  "slur5",
  // sexual
  "porn",
  "nsfw",
  "horny",
  "sexy",
  "naked",
  "boobs",
  // violent
  "kill",
  "murder",
  "rape",
  "shoot",
  "stab",
  "bomb",
  "lynch",
  // self-harm
  "suicide",
  "kys",
  // hate speech umbrella
  "nazi",
  "kkk",
  "terrorist",
  // brand-unsafe operator-facing phrases
  "scam",
  "fraud",
  "ponzi",
];

/** Lower-cased Set for O(1) substring screening. */
export const BLOCKED_PHRASE_TOKENS: ReadonlySet<string> = new Set(
  RAW_TOKENS.map((t) => t.toLowerCase()),
);
