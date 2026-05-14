/**
 * Tiny client-side cron parser/validator for the Step 4 structured
 * Adjust drawer.
 *
 * Subset supported (per E2-f brief):
 *   minute  hour  day-of-month  month  day-of-week
 *   field tokens: N | * | * / N | A,B,C  (A,B,C may include ranges)
 *
 * No special tokens (`@daily`, etc.) — minimal surface for UI
 * validation. Server-side validation lives behind
 * `mission-enterprise-duty-cron-validation` (the architectural
 * target); v1 trusts the client for friendliness only.
 */
export interface CronFieldSpec {
  /** Inclusive minimum value for the field. */
  min: number;
  /** Inclusive maximum value for the field. */
  max: number;
  /** Human-readable name (used in error messages). */
  name: string;
}

export const CRON_FIELDS: readonly CronFieldSpec[] = [
  { min: 0, max: 59, name: "minute" },
  { min: 0, max: 23, name: "hour" },
  { min: 1, max: 31, name: "day-of-month" },
  { min: 1, max: 12, name: "month" },
  { min: 0, max: 6, name: "day-of-week" },
] as const;

export interface CronParseOk {
  ok: true;
}

export interface CronParseErr {
  ok: false;
  /** Human-readable reason; safe to render inline. */
  reason: string;
}

export type CronParseResult = CronParseOk | CronParseErr;

/**
 * Parse and validate a 5-field cron expression. Returns `{ ok: true }`
 * if the expression conforms to the supported subset, otherwise
 * `{ ok: false, reason }` with a short message suitable for inline UI.
 *
 * Whitespace around fields is tolerated; internal whitespace inside a
 * field is rejected.
 */
export function parseCron(expression: string): CronParseResult {
  if (typeof expression !== "string") {
    return { ok: false, reason: "Expected a cron string" };
  }
  const trimmed = expression.trim();
  if (trimmed.length === 0) {
    return { ok: false, reason: "Cron expression is empty" };
  }
  const fields = trimmed.split(/\s+/);
  if (fields.length !== 5) {
    return {
      ok: false,
      reason: `Expected 5 fields, got ${fields.length}`,
    };
  }
  for (let i = 0; i < 5; i += 1) {
    const field = fields[i]!;
    const spec = CRON_FIELDS[i]!;
    const result = parseField(field, spec);
    if (!result.ok) return result;
  }
  return { ok: true };
}

function parseField(field: string, spec: CronFieldSpec): CronParseResult {
  if (field.length === 0) {
    return { ok: false, reason: `Empty ${spec.name} field` };
  }
  // List form: A,B,C — each part may be a range, step, *, or single.
  if (field.includes(",")) {
    const parts = field.split(",");
    for (const part of parts) {
      if (part.length === 0) {
        return {
          ok: false,
          reason: `Empty value in ${spec.name} list`,
        };
      }
      const res = parseAtom(part, spec);
      if (!res.ok) return res;
    }
    return { ok: true };
  }
  return parseAtom(field, spec);
}

function parseAtom(atom: string, spec: CronFieldSpec): CronParseResult {
  // Step form: */N or A-B/N or A/N
  if (atom.includes("/")) {
    const [base, stepRaw, ...rest] = atom.split("/");
    if (rest.length > 0 || base === undefined || stepRaw === undefined) {
      return {
        ok: false,
        reason: `Invalid step in ${spec.name}: "${atom}"`,
      };
    }
    if (!/^[1-9][0-9]*$/.test(stepRaw)) {
      return {
        ok: false,
        reason: `Step must be a positive integer in ${spec.name}: "${atom}"`,
      };
    }
    const step = Number.parseInt(stepRaw, 10);
    if (step > spec.max) {
      return {
        ok: false,
        reason: `Step ${step} exceeds ${spec.name} max ${spec.max}`,
      };
    }
    if (base === "*") return { ok: true };
    return parseRangeOrNumber(base, spec);
  }
  if (atom === "*") return { ok: true };
  return parseRangeOrNumber(atom, spec);
}

function parseRangeOrNumber(
  atom: string,
  spec: CronFieldSpec,
): CronParseResult {
  if (atom.includes("-")) {
    const [lo, hi, ...rest] = atom.split("-");
    if (rest.length > 0 || lo === undefined || hi === undefined) {
      return {
        ok: false,
        reason: `Invalid range in ${spec.name}: "${atom}"`,
      };
    }
    const loRes = parseNumber(lo, spec);
    if (!loRes.ok) return loRes;
    const hiRes = parseNumber(hi, spec);
    if (!hiRes.ok) return hiRes;
    const loN = Number.parseInt(lo, 10);
    const hiN = Number.parseInt(hi, 10);
    if (loN > hiN) {
      return {
        ok: false,
        reason: `Range ${loN}-${hiN} is reversed in ${spec.name}`,
      };
    }
    return { ok: true };
  }
  return parseNumber(atom, spec);
}

function parseNumber(token: string, spec: CronFieldSpec): CronParseResult {
  if (!/^[0-9]+$/.test(token)) {
    return {
      ok: false,
      reason: `Invalid ${spec.name} value: "${token}"`,
    };
  }
  const n = Number.parseInt(token, 10);
  if (n < spec.min || n > spec.max) {
    return {
      ok: false,
      reason: `${spec.name} ${n} out of range ${spec.min}-${spec.max}`,
    };
  }
  return { ok: true };
}

/** Friendly preset chips surfaced inline on the schedule editor. */
export interface CronPreset {
  /** Stable id used as the React key + data-testid suffix. */
  id: string;
  /** Label shown on the chip button. */
  label: string;
  /** Cron expression applied when the chip is clicked. */
  spec: string;
}

export const CRON_PRESETS: readonly CronPreset[] = [
  { id: "morning", label: "Every morning", spec: "0 6 * * *" },
  { id: "hourly", label: "Every hour", spec: "0 * * * *" },
  { id: "weekdays-9am", label: "Weekdays 9am", spec: "0 9 * * 1-5" },
  { id: "weekly-mon", label: "Once a week (Mon)", spec: "0 9 * * 1" },
] as const;
