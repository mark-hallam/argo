/**
 * Operator-settable pronoun helper. The persona spine carries an
 * explicit `pronouns` string (e.g. "she/her", "he/him", "they/them")
 * and copy that referred to a worker as "she" or "he" calls
 * `formatPronouns()` instead of hardcoding the form.
 *
 * Default fall-through is `they/them` so a missing or unrecognised
 * value reads as inclusive rather than gendered-by-name.
 *
 * Rendering tip: `Sentence-initial` use needs the capitalised form.
 * `She lists items.` becomes `${cap(p.subject)} lists items.` via
 * the `Cap` accessors below.
 */

export type PronounSet = "she/her" | "he/him" | "they/them" | string;

export interface Pronouns {
  /** Subject pronoun — "she" / "he" / "they". */
  subject: string;
  /** Object pronoun — "her" / "him" / "them". */
  object: string;
  /** Possessive determiner — "her" / "his" / "their". */
  possessive: string;
  /** Reflexive pronoun — "herself" / "himself" / "themself". */
  reflexive: string;
  /** Sentence-initial capitalised subject — "She" / "He" / "They". */
  subjectCap: string;
  /** Whether the subject takes a plural verb form. `they` does. */
  isPlural: boolean;
}

const THEY: Pronouns = {
  subject: "they",
  object: "them",
  possessive: "their",
  reflexive: "themself",
  subjectCap: "They",
  isPlural: true,
};

const SHE: Pronouns = {
  subject: "she",
  object: "her",
  possessive: "her",
  reflexive: "herself",
  subjectCap: "She",
  isPlural: false,
};

const HE: Pronouns = {
  subject: "he",
  object: "him",
  possessive: "his",
  reflexive: "himself",
  subjectCap: "He",
  isPlural: false,
};

export function formatPronouns(set: PronounSet | undefined | null): Pronouns {
  switch ((set ?? "they/them").toLowerCase()) {
    case "she/her":
      return SHE;
    case "he/him":
      return HE;
    default:
      return THEY;
  }
}
