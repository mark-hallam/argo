/**
 * Worker-trait helpers.
 *
 * The live gateway's demo-seed (`crates/argo-demo-seed/src/seed.rs`)
 * builds each worker's role line as:
 *
 *   "{title}: {tagline}. Traits: {t1}, {t2}, {t3}, {t4}."
 *
 * i.e. the worker's REAL personality traits are appended to the
 * `role.tagline` the gateway returns on `GET /api/v1/workers/<id>`.
 * That suffix is the authoritative, per-worker trait source available
 * client-side — the seed and the SPA persona spine
 * (`web/lib/sample-personas.ts`) both derive it from the same
 * `DEMO_SEED_PERSONAS` array, so parsing it is reading real data, not
 * fabricating it.
 *
 * Why this matters: in demo-live the gateway re-seeds workers with
 * fresh UUIDs that don't match the hardcoded ids in
 * `sample-personas.ts`, so a UUID-keyed `findSamplePersona(id)` lookup
 * misses and the personality card used to fall back to generic
 * boilerplate ("Reliable / Curious / Helpful"). Parsing the tagline
 * recovers the real traits regardless of which UUID the gateway minted.
 *
 * `parseTraitsFromTagline` is intentionally forgiving: a tagline with
 * no `Traits:` suffix (a hand-typed hire, an older seed) returns an
 * empty array so the caller can fall through to its next source.
 */

/** Marker the demo-seed uses to delimit the trait list in a tagline. */
const TRAITS_MARKER = "Traits:";

/**
 * Pull the comma-separated trait list out of a worker's role tagline.
 *
 * Returns `[]` when the tagline carries no `Traits:` suffix so callers
 * can fall through to another source (the persona spine, then a
 * generic default) rather than rendering an empty chip row.
 *
 * @example
 * parseTraitsFromTagline(
 *   "Marketing: drafts replies. Traits: On-voice, Truth-bounded, Draft-only.",
 * );
 * // → ["On-voice", "Truth-bounded", "Draft-only"]
 *
 * @example
 * parseTraitsFromTagline("books showings"); // → []
 */
export function parseTraitsFromTagline(
  tagline: string | undefined | null,
): string[] {
  if (!tagline) return [];
  const markerIndex = tagline.lastIndexOf(TRAITS_MARKER);
  if (markerIndex === -1) return [];

  // Everything after "Traits:" up to the trailing period the seed adds.
  const raw = tagline.slice(markerIndex + TRAITS_MARKER.length).trim();
  const withoutTrailingDot = raw.endsWith(".") ? raw.slice(0, -1) : raw;

  return withoutTrailingDot
    .split(",")
    .map((trait) => trait.trim())
    .filter((trait) => trait.length > 0);
}

/**
 * Strip the `". Traits: …"` suffix off a tagline so the bare role
 * description can be shown on its own (the traits are rendered
 * separately as chips). Returns the tagline unchanged when no suffix
 * is present.
 *
 * @example
 * stripTraitsSuffix("Recruiter: books interviews. Traits: Fast, Calm.");
 * // → "Recruiter: books interviews."
 */
export function stripTraitsSuffix(
  tagline: string | undefined | null,
): string {
  if (!tagline) return "";
  const markerIndex = tagline.lastIndexOf(TRAITS_MARKER);
  if (markerIndex === -1) return tagline.trim();

  // Drop the marker and any ". " / " " that precedes it.
  return tagline
    .slice(0, markerIndex)
    .replace(/[.\s]+$/, ".")
    .trim();
}
