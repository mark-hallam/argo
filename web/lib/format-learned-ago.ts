/**
 * Plain-language "Learned 2 weeks ago" formatter for the learning-loop
 * chip. The brief mandates the verb "Learned" (not "Acquired") and an
 * approachable, non-jargon register — operator memory
 * `user_plain_language_no_jargon.md`.
 *
 * Granularity is coarse on purpose: hour-level precision would
 * encourage operators to read the gateway clock; "this week" / "2
 * weeks ago" / "last month" is the right register for a marketplace
 * surface where the absolute moment doesn't matter.
 *
 * Pure function — `now` is injected so tests can pin a reference
 * point without touching the wall clock.
 */
export function formatLearnedAgo(
  learnedAtIso: string | null | undefined,
  now: Date = new Date(),
): string | null {
  if (!learnedAtIso) return null;
  const learnedAt = new Date(learnedAtIso);
  if (Number.isNaN(learnedAt.getTime())) return null;

  const diffMs = now.getTime() - learnedAt.getTime();
  if (diffMs < 0) return "Learned just now";

  const minute = 60 * 1000;
  const hour = 60 * minute;
  const day = 24 * hour;
  const week = 7 * day;

  if (diffMs < hour) return "Learned this hour";
  if (diffMs < day) {
    const hours = Math.floor(diffMs / hour);
    return hours === 1 ? "Learned 1 hour ago" : `Learned ${hours} hours ago`;
  }
  if (diffMs < week) {
    const days = Math.floor(diffMs / day);
    return days === 1 ? "Learned yesterday" : `Learned ${days} days ago`;
  }
  // Same calendar month → "Learned N weeks ago"; older → "Learned in <Mon YYYY>"
  if (
    now.getUTCFullYear() === learnedAt.getUTCFullYear() &&
    now.getUTCMonth() === learnedAt.getUTCMonth()
  ) {
    const weeks = Math.floor(diffMs / week);
    return weeks <= 1 ? "Learned 1 week ago" : `Learned ${weeks} weeks ago`;
  }
  if (diffMs < 4 * week + day) {
    return "Learned 4 weeks ago";
  }
  const month = learnedAt.toLocaleString("en-AU", { month: "short", year: "numeric", timeZone: "UTC" });
  return `Learned in ${month}`;
}
