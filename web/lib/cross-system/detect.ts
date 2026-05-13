/**
 * Cross-system reference detector (v1).
 *
 * Walks the registered generators over a source string and returns
 * a sorted, non-overlapping list of `CrossSystemMatch` entries.
 * Each entry has been validated against the issuing generator's
 * `verifyOrigin` — generator regex bugs that emit out-of-allowlist
 * URLs are dropped here, not surfaced to the renderer.
 *
 * Overlap policy: later matches yield to earlier ones. If two
 * generators independently match overlapping regions, the earlier
 * (by start offset) wins; ties break on registry order.
 */
import type { ConnectorGenerator, CrossSystemMatch } from "./types";
import { getRegisteredGenerators } from "./registry";

const MAX_LENGTH = 4000;

/** Detect cross-system references in a freeform text payload.
 *
 *  Bounded length: payloads above `MAX_LENGTH` are truncated before
 *  detection runs (the un-scanned tail is discarded). 4 KB covers
 *  every realistic audit-row body while keeping detect under 10 ms
 *  for the 30+ matches per row upper bound the brief calls out. */
export function detect(text: string): CrossSystemMatch[] {
  if (!text || typeof text !== "string") return [];
  const truncated = text.length > MAX_LENGTH ? text.slice(0, MAX_LENGTH) : text;
  const generators = getRegisteredGenerators();
  const raw: CrossSystemMatch[] = [];
  for (const gen of generators) {
    for (const m of safeMatch(gen, truncated)) {
      if (!gen.verifyOrigin(m.deepLink.url)) continue;
      raw.push({
        start: m.start,
        end: m.end,
        system: gen.system,
        raw: m.raw,
        deepLink: m.deepLink,
      });
    }
  }
  return resolveOverlaps(raw);
}

function safeMatch(gen: ConnectorGenerator, text: string) {
  try {
    return gen.match(text);
  } catch {
    return [];
  }
}

/** Sort matches by start offset and drop any that overlap a
 *  previously kept earlier match. Stable on registry order via the
 *  secondary tiebreaker. */
function resolveOverlaps(matches: CrossSystemMatch[]): CrossSystemMatch[] {
  if (matches.length <= 1) return matches.slice();
  const sorted = matches.slice().sort((a, b) => a.start - b.start || a.end - b.end);
  const kept: CrossSystemMatch[] = [];
  for (const m of sorted) {
    if (kept.length === 0) {
      kept.push(m);
      continue;
    }
    const last = kept[kept.length - 1];
    if (m.start < last.end) continue;
    kept.push(m);
  }
  return kept;
}

/** Split text against the supplied matches, yielding a sequence of
 *  `{ kind: "text", value }` and `{ kind: "match", match }` segments
 *  the renderer iterates. Pure function; no React imports. */
export type DetectSegment =
  | { kind: "text"; value: string }
  | { kind: "match"; match: CrossSystemMatch };

export function segmentText(
  text: string,
  matches: CrossSystemMatch[],
): DetectSegment[] {
  if (matches.length === 0) {
    return text ? [{ kind: "text", value: text }] : [];
  }
  const out: DetectSegment[] = [];
  let cursor = 0;
  for (const m of matches) {
    if (m.start > cursor) {
      out.push({ kind: "text", value: text.slice(cursor, m.start) });
    }
    out.push({ kind: "match", match: m });
    cursor = m.end;
  }
  if (cursor < text.length) {
    out.push({ kind: "text", value: text.slice(cursor) });
  }
  return out;
}
