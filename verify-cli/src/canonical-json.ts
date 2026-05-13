/**
 * Canonical JSON serialization for byte-stable hashing.
 *
 * Algorithm:
 *   1. Object keys sorted lexicographically (recursive).
 *   2. No insignificant whitespace.
 *   3. Numbers use JSON.stringify normalization (no leading zeros,
 *      exponents in canonical form per RFC 7159).
 *   4. String escaping is delegated to JSON.stringify, which emits
 *      the minimal required JSON escapes.
 *
 * Non-finite numbers (NaN, Infinity) are not representable in JSON
 * and are rejected to avoid emitting invalid output.
 */

export function canonicalize(value: unknown): string {
  return stringify(value);
}

function stringify(value: unknown): string {
  if (value === null) return "null";

  const t = typeof value;

  if (t === "boolean") return value ? "true" : "false";

  if (t === "number") {
    const n = value as number;
    if (!Number.isFinite(n)) {
      throw new Error(`canonicalize: non-finite number ${String(n)}`);
    }
    return JSON.stringify(n);
  }

  if (t === "string") return JSON.stringify(value as string);

  if (Array.isArray(value)) {
    const items = value.map((v) => stringify(v));
    return `[${items.join(",")}]`;
  }

  if (t === "object") {
    const obj = value as Record<string, unknown>;
    const keys = Object.keys(obj)
      .filter((k) => obj[k] !== undefined)
      .sort();
    const parts = keys.map(
      (k) => `${JSON.stringify(k)}:${stringify(obj[k])}`
    );
    return `{${parts.join(",")}}`;
  }

  if (t === "undefined") {
    throw new Error("canonicalize: undefined is not representable");
  }

  throw new Error(`canonicalize: unsupported type ${t}`);
}
