/**
 * Deterministic English-cadence → cron mapper for the Genesis hire path
 * (DEC-048 rule 2 — captured-equals-built). The Genesis intake captures WHEN
 * the teammate runs as the operator's own free-text words (e.g. "every weekday
 * at 5pm"); this turns those words into the 5-field wire cron the hire draft
 * sends so the minted duty fires when the operator asked — never a hardcoded
 * 9am default.
 *
 * It is NOT a parallel cron stack: the cron string we emit is validated by the
 * canonical client validator `parseCron` (`@/lib/cron/parse`) before it leaves
 * here, and the server re-normalizes it with `normalize_stored_cron`
 * (`routes_duties_helpers.rs`) into the 6-field stored form. We only add the
 * English→cron mapping, which no existing helper provides.
 *
 * Fail-soft, never silent: an unrecognized cadence returns `parsed: false` +
 * the default cron, so the caller keeps the operator's words in the duty
 * description and the review card flags "schedule needs confirmation" rather
 * than silently substituting a schedule the operator never asked for.
 */
import { parseCron } from "@/lib/cron/parse";

/** The fallback cron used when a cadence can't be parsed (weekdays 9am). */
export const DEFAULT_CADENCE_CRON = "0 9 * * 1-5";

export interface ParsedCadence {
  /** A valid 5-field cron the hire draft sends. Always valid; the default
   *  when the words couldn't be parsed. */
  cron: string;
  /** `true` when a schedule was recognized in the words; `false` when we fell
   *  back to {@link DEFAULT_CADENCE_CRON} (caller surfaces "needs confirmation"). */
  parsed: boolean;
}

const DAY_OF_WEEK: Record<string, number> = {
  sun: 0,
  sunday: 0,
  mon: 1,
  monday: 1,
  tue: 2,
  tues: 2,
  tuesday: 2,
  wed: 3,
  weds: 3,
  wednesday: 3,
  thu: 4,
  thur: 4,
  thurs: 4,
  thursday: 4,
  fri: 5,
  friday: 5,
  sat: 6,
  saturday: 6,
};

/** Extract an hour (0-23) + minute from a phrase, or `null` when none. */
function extractTime(text: string): { hour: number; minute: number } | null {
  // 1) Meridiem time: "5pm", "5 pm", "5:30pm", "5 p.m.".
  const mer = text.match(/\b(\d{1,2})(?::(\d{2}))?\s*(a\.?m\.?|p\.?m\.?)/);
  if (mer) {
    let hour = Number.parseInt(mer[1]!, 10);
    const minute = mer[2] ? Number.parseInt(mer[2], 10) : 0;
    const isPm = mer[3]!.toLowerCase().startsWith("p");
    if (isPm && hour < 12) hour += 12;
    if (!isPm && hour === 12) hour = 0;
    if (hour <= 23 && minute <= 59) return { hour, minute };
  }
  // 2) 24-hour colon time: "17:00", "at 9:30".
  const colon = text.match(/\b(\d{1,2}):(\d{2})\b/);
  if (colon) {
    const hour = Number.parseInt(colon[1]!, 10);
    const minute = Number.parseInt(colon[2]!, 10);
    if (hour <= 23 && minute <= 59) return { hour, minute };
  }
  // 3) Bare "at <n>" hour: "at 9", "at 17".
  const at = text.match(/\bat\s+(\d{1,2})\b/);
  if (at) {
    const hour = Number.parseInt(at[1]!, 10);
    if (hour <= 23) return { hour, minute: 0 };
  }
  return null;
}

/** Resolve the day-of-week cron field + whether a day pattern was recognized. */
function extractDayOfWeek(text: string): { field: string; matched: boolean } {
  if (/\bweekend/.test(text)) return { field: "0,6", matched: true };
  if (/\bweekday/.test(text)) return { field: "1-5", matched: true };
  if (/\b(every\s*day|everyday|daily)\b/.test(text)) {
    return { field: "*", matched: true };
  }
  const days = Object.keys(DAY_OF_WEEK)
    .filter((name) => new RegExp(`\\b${name}\\b`).test(text))
    .map((name) => DAY_OF_WEEK[name]!);
  if (days.length > 0) {
    const unique = Array.from(new Set(days)).sort((a, b) => a - b);
    return { field: unique.join(","), matched: true };
  }
  // "weekly" with no named day → default to Monday.
  if (/\bweekly\b/.test(text)) return { field: "1", matched: true };
  return { field: "*", matched: false };
}

/**
 * Map a free-text cadence to a validated 5-field cron. Returns the default
 * cron with `parsed: false` for anything we can't confidently recognize, so
 * the caller never silently fabricates a schedule.
 */
export function parseCadenceToCron(
  cadence: string | null | undefined,
): ParsedCadence {
  const text = (cadence ?? "").toLowerCase().trim();
  if (text.length === 0) return { cron: DEFAULT_CADENCE_CRON, parsed: false };

  // "hourly" / "every hour" is a whole-field special case.
  if (/\b(hourly|every\s*hour)\b/.test(text)) {
    return { cron: "0 * * * *", parsed: true };
  }

  const time = extractTime(text);
  const dow = extractDayOfWeek(text);
  if (!time && !dow.matched) {
    return { cron: DEFAULT_CADENCE_CRON, parsed: false };
  }

  const minute = time ? time.minute : 0;
  const hour = time ? time.hour : 9; // a day pattern but no time → 9am.
  const cron = `${minute} ${hour} * * ${dow.field}`;
  // Reuse the canonical client validator — never emit a cron it rejects.
  if (!parseCron(cron).ok) {
    return { cron: DEFAULT_CADENCE_CRON, parsed: false };
  }
  return { cron, parsed: true };
}
