/**
 * Render a KPI numeric value with consistent zero/undefined vocabulary.
 *
 * Picks `0` vs `—` deliberately:
 *   - Counts / sums always have a value, even when empty (0, $0). Pass a
 *     non-null number — the formatter renders the canonical zero.
 *   - Averages over 0 events are mathematically undefined. Pass `null` /
 *     `undefined` so the dash fallback fires.
 *   - Rates over 0 events: if you treat "no decisions" as `0%`, pass `0`;
 *     if you want to flag absent data, pass `null`.
 *
 * Without this helper, callers tend to mix `value > 0 ? format(value) : "—"`
 * (which renders the dash for genuine zeros) and `format(value)` (which
 * renders 0 even when no data exists). The result is the same KPI strip
 * showing `Pending: 0` next to `Resolved/wk: —` — inconsistent vocabulary
 * for what are both "empty" states.
 */
export function formatKpi(
  value: number | null | undefined,
  format: (n: number) => string,
  fallback = "—",
): string {
  if (value == null || !Number.isFinite(value)) return fallback;
  return format(value);
}
