/**
 * Small JSON helpers for the workflow builder's body editors.
 *
 * Native `JSON.parse` only — no deps. Browser engines (V8/JSC) report
 * parse positions as `position N` rather than line/column, so the
 * line-number derivation here is a best-effort scan of the source up
 * to that offset.
 */

export type JsonParseResult =
  | { ok: true; value: unknown }
  | { ok: false; message: string; line: number };

/**
 * Returns true when the content's first non-whitespace character is
 * `{` or `[` — the heuristic the brief calls out for "looks like
 * JSON". Empty / whitespace-only content returns false so the editor
 * doesn't flag an empty body as a parse error.
 */
export function looksLikeJson(input: string): boolean {
  const trimmed = input.trimStart();
  if (trimmed.length === 0) return false;
  const first = trimmed[0];
  return first === "{" || first === "[";
}

/**
 * Native-`JSON.parse` wrapper. On failure, extracts a `position N`
 * offset from the engine's error message (V8/JSC convention) and
 * converts it to a 1-based line number by counting newlines in the
 * source up to that offset. If no offset is found, line defaults to 1.
 */
export function tryParseJson(input: string): JsonParseResult {
  try {
    return { ok: true, value: JSON.parse(input) };
  } catch (err) {
    const raw = err instanceof Error ? err.message : String(err);
    const match = raw.match(/position\s+(\d+)/i);
    let line = 1;
    if (match) {
      const offset = Number.parseInt(match[1] ?? "0", 10);
      if (Number.isFinite(offset) && offset > 0) {
        line = 1;
        for (let i = 0; i < offset && i < input.length; i++) {
          if (input[i] === "\n") line += 1;
        }
      }
    }
    return { ok: false, message: raw, line };
  }
}

/**
 * Pretty-prints `input` with 2-space indentation when it parses as
 * valid JSON AND looks like JSON. Returns `null` for empty,
 * non-JSON-shaped, or invalid input so the caller can leave the
 * existing value untouched.
 */
export function prettyPrintJson(input: string): string | null {
  if (!looksLikeJson(input)) return null;
  const parsed = tryParseJson(input);
  if (!parsed.ok) return null;
  return JSON.stringify(parsed.value, null, 2);
}
